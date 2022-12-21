use async_trait::async_trait;
use futures::{future::LocalBoxFuture, pin_mut, stream::FusedStream, FutureExt, Stream, StreamExt};
use hass_dyn_error::DynError;
use hass_mqtt_provider::{
	AsMqttOptions, MqttBuildableMessage, MqttClient, MqttDisconnectBuilder, MqttMessage,
	MqttMessageBuilder, MqttOptions, MqttProvider, MqttProviderCreateError, MqttPublishBuilder,
	MqttReceivedMessage, MqttRetainHandling, MqttSubscribeBuilder, MqttUnsubscribeBuilder, QosLevel,
};
use pin_project::pin_project;
use std::{
	cell::RefCell,
	convert::Infallible,
	future::IntoFuture,
	pin::Pin,
	sync::Arc,
	task::{Context, Poll},
	time::Duration,
};
use thiserror::Error;
use tokio::{net::lookup_host, task};
use tracing::{event, instrument, span, Instrument, Level, Span};

// https://github.com/eclipse/paho.mqtt.rust/issues/182
trait PahoClientExt {
	fn set_connected_callback_safe<F>(&self, cb: F)
	where
		F: FnMut(&paho_mqtt::AsyncClient) + Send + Sync + 'static;

	fn set_connection_lost_callback_safe<F>(&self, cb: F)
	where
		F: FnMut(&paho_mqtt::AsyncClient) + Send + Sync + 'static;

	fn set_disconnected_callback_safe<F>(&self, cb: F)
	where
		F: FnMut(&paho_mqtt::AsyncClient, paho_mqtt::Properties, paho_mqtt::ReasonCode)
			+ Send
			+ Sync
			+ 'static;

	fn set_message_callback_safe<F>(&self, cb: F)
	where
		F: FnMut(&paho_mqtt::AsyncClient, Option<paho_mqtt::Message>) + Send + Sync + 'static;
}

impl PahoClientExt for paho_mqtt::AsyncClient {
	fn set_connected_callback_safe<F>(&self, cb: F)
	where
		F: FnMut(&paho_mqtt::AsyncClient) + Send + Sync + 'static,
	{
		self.set_connected_callback(cb)
	}

	fn set_connection_lost_callback_safe<F>(&self, cb: F)
	where
		F: FnMut(&paho_mqtt::AsyncClient) + Send + Sync + 'static,
	{
		self.set_connection_lost_callback(cb)
	}

	fn set_disconnected_callback_safe<F>(&self, cb: F)
	where
		F: FnMut(&paho_mqtt::AsyncClient, paho_mqtt::Properties, paho_mqtt::ReasonCode)
			+ Send
			+ Sync
			+ 'static,
	{
		self.set_disconnected_callback(cb)
	}

	fn set_message_callback_safe<F>(&self, cb: F)
	where
		F: FnMut(&paho_mqtt::AsyncClient, Option<paho_mqtt::Message>) + Send + Sync + 'static,
	{
		self.set_message_callback(cb)
	}
}

fn create_callback<F, Args, RetFut>(mut f: F) -> impl FnMut(Args) + Send + Sync
where
	F: FnMut(Args) -> RetFut + 'static,
	RetFut: IntoFuture<Output = ()>,
	Args: Send + 'static,
{
	let (sender, receiver) = flume::bounded::<Args>(0);
	task::spawn_local(async move {
		let stream = receiver.into_stream();
		pin_mut!(stream);

		while let Some(args) = stream.next().await {
			f(args).await;
		}
	});

	move |args| {
		sender.send(args).unwrap();
	}
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum PahoProviderConnectError {
	#[error("failed to create MQTT client")]
	Client {
		#[cfg_attr(provide_any, backtrace)]
		source: DynError,
	},

	#[error("failed to connect to MQTT broker")]
	Connect {
		#[cfg_attr(provide_any, backtrace)]
		source: DynError,
	},

	#[error("falied to resolve host: {host}:{port}")]
	ResolveHost {
		host: String,
		port: u16,
		#[cfg_attr(provide_any, backtrace)]
		source: DynError,
	},

	#[error("failed to create MQTT message: {kind}")]
	Message {
		kind: String,
		#[cfg_attr(provide_any, backtrace)]
		source: DynError,
	},
}

impl PahoProviderConnectError {
	fn client(source: impl std::error::Error + Send + Sync + 'static) -> Self {
		Self::Client {
			source: DynError::new(source),
		}
	}

	fn connect(source: impl std::error::Error + Send + Sync + 'static) -> Self {
		Self::Connect {
			source: DynError::new(source),
		}
	}

	fn resolve_host(
		host: impl Into<String>,
		port: u16,
		source: impl std::error::Error + Send + Sync + 'static,
	) -> Self {
		Self::ResolveHost {
			host: host.into(),
			port,
			source: DynError::new(source),
		}
	}

	fn message(
		kind: impl Into<String>,
		source: impl std::error::Error + Send + Sync + 'static,
	) -> Self {
		Self::Message {
			kind: kind.into(),
			source: DynError::new(source),
		}
	}
}

impl MqttProviderCreateError for PahoProviderConnectError {
	fn create_message(
		kind: impl Into<String>,
		source: impl std::error::Error + Send + Sync + 'static,
	) -> Self {
		Self::message(kind, source)
	}
}

pub struct PahoMqtt;

#[async_trait(?Send)]
impl MqttProvider for PahoMqtt {
	const NAME: &'static str = "paho";

	type Client = Client;
	type Message = Message;
	type Error = PahoProviderConnectError;

	#[instrument(
		level = Level::DEBUG,
		name = "PahoMqtt::create",
		skip_all,
		fields(
			client.id = %client_id,
		),
		err,
	)]
	async fn create(
		options: &impl AsMqttOptions,
		client_id: &str,
		online_message: Self::Message,
		offline_message: Self::Message,
	) -> Result<Self::Client, Self::Error> {
		let options = options
			.mqtt_options()
			.map_err(|e| PahoProviderConnectError::message("failed to create MQTT options", e))?;

		let client = paho_mqtt::AsyncClient::new(as_create_options(&options, client_id)?)
			.map_err(PahoProviderConnectError::client)?;

		let mut builder = paho_mqtt::ConnectOptionsBuilder::new();
		let hosts = lookup_host((&*options.host, options.port))
			.instrument(
				span!(Level::DEBUG, "PahoMqtt::lookup_host", host = %options.host, port = options.port),
			)
			.await
			.map_err(|source| {
				PahoProviderConnectError::resolve_host(&options.host, options.port, source)
			})?
			.map(|addr| format!("tcp://{addr}"))
			.collect::<Vec<_>>();

		builder
			.server_uris(&hosts)
			.automatic_reconnect(Duration::from_secs(5), Duration::from_secs(60 * 5))
			.mqtt_version(0);

		#[cfg(feature = "tls")]
		if options.tls {
			builder.ssl_options(paho_mqtt::SslOptions::new());
		}

		if let Some(auth) = &options.auth {
			builder.user_name(auth.username.clone());
			builder.password(auth.password.clone());
		}

		let span = Span::current();
		let (message_sender, message_receiver) = flume::unbounded();
		let inner = InnerClient::new(client.clone(), message_receiver);

		builder.will_message(offline_message.message);

		let mut connected_callback = create_callback({
			let inner = inner.clone();
			let span = span.clone();
			move |_: ()| {
				let inner = inner.clone();
				let span = span.clone();
				let online_message = online_message.clone();
				async move {
					let client = &inner.client;
					let span = span.clone();
					let client_id = client.client_id();
					let mqtt_version = client.mqtt_version();

					let subscriptions = inner.subscriptions.borrow();
					let subscriptions = &*subscriptions;
					if !subscriptions.is_empty() {
						let mut topics = Vec::with_capacity(subscriptions.len());
						let mut qos = Vec::with_capacity(subscriptions.len());
						let mut options = Vec::with_capacity(subscriptions.len());
						for opt in subscriptions {
							topics.push(opt.topic.clone());
							qos.push(i32::from(opt.qos));
							options.push(paho_mqtt::SubscribeOptions::from(opt));
						}

						if let Err(e) = client
							.subscribe_many_with_options(&topics, &qos, &options, None)
							.await
						{
							event!(
								parent: &span,
								Level::ERROR,
								client.id = %client_id,
								client.mqtt.version = %mqtt_version,
								"failed to resubscribe to topics: {:#}",
								e,
							);
						}
					}

					if let Err(e) = client.publish(online_message.message).await {
						event!(
							parent: &span,
							Level::ERROR,
							client.id = %client_id,
							client.mqtt.version = %mqtt_version,
							"failed to publish online message: {:#}",
							e,
						);
					}
				}
			}
		});

		let mut connection_lost_callback = create_callback({
			let span = span.clone();
			let inner = inner.clone();
			move |_: ()| {
				let span = span.clone();
				let client_id = inner.client.client_id();
				let mqtt_version = inner.client.mqtt_version();
				async move {
					event!(
						parent: &span,
						Level::WARN,
						client.id = %client_id,
						client.mqtt.version = %mqtt_version,
						"connection lost");
				}
			}
		});

		let mut disconnected_callback = create_callback({
			let span = span.clone();
			let inner = inner.clone();
			move |(reason,): (paho_mqtt::ReasonCode,)| {
				let span = span.clone();
				let client_id = inner.client.client_id();
				let mqtt_version = inner.client.mqtt_version();
				async move {
					event!(
						parent: &span,
						Level::WARN,
						client.id = %client_id,
						client.mqtt.version = %mqtt_version,
						reason = %reason,
						"disconnected");
				}
			}
		});

		let mut message_callback = create_callback({
			let span = span.clone();
			let inner = inner.clone();
			move |(message,): (Option<paho_mqtt::Message>,)| {
				let span = span.clone();
				let client_id = inner.client.client_id();
				let mqtt_version = inner.client.mqtt_version();
				let message_sender = message_sender.clone();
				async move {
					if let Some(message) = message {
						event!(
							parent: &span,
							Level::DEBUG,
							client.id = %client_id,
							client.mqtt.version = %mqtt_version,
							message.topic = %message.topic(),
							message.retained = message.retained(),
							message.qos = %message.qos(),
							message.payload.len = message.payload().len(),
						);
						if let Err(e) = message_sender.send_async((message, span.clone())).await {
							event!(
								parent: &span,
								Level::ERROR,
								client.id = %client_id,
								"failed to send message to listeners: {:#}",
								e,
							);
						}
					}
				}
			}
		});

		client.set_connected_callback_safe(move |_| connected_callback(()));
		client.set_connection_lost_callback_safe(move |_| connection_lost_callback(()));
		client.set_disconnected_callback(move |_, _props, reason| disconnected_callback((reason,)));
		client.set_message_callback(move |_, message| message_callback((message,)));

		client
			.connect(builder.finalize())
			.instrument(span!(Level::DEBUG, "PahoMqtt::connect", client.id = %client_id))
			.await
			.map_err(PahoProviderConnectError::connect)?;

		Ok(Client { inner })
	}
}

#[derive(Clone)]
struct SubscriptionOptions {
	topic: Arc<str>,
	qos: QosLevel,
	no_local: Option<bool>,
	retain_handling: Option<MqttRetainHandling>,
}

impl SubscriptionOptions {
	pub fn is_empty(&self) -> bool {
		self.no_local.is_none() && self.retain_handling.is_none()
	}
}

impl From<SubscribeBuilder<'_>> for SubscriptionOptions {
	fn from(value: SubscribeBuilder<'_>) -> Self {
		Self {
			topic: value.topic,
			qos: value.qos,
			no_local: value.no_local,
			retain_handling: value.retain_handling,
		}
	}
}

impl From<&SubscriptionOptions> for paho_mqtt::SubscribeOptions {
	fn from(value: &SubscriptionOptions) -> Self {
		let mut options = paho_mqtt::SubscribeOptionsBuilder::new();
		if let Some(no_local) = value.no_local {
			options.no_local(no_local);
		}

		if let Some(retain_handling) = value.retain_handling {
			options.retain_handling(match retain_handling {
				MqttRetainHandling::SendRetainedOnSubscribe => {
					paho_mqtt::RetainHandling::SendRetainedOnSubscribe
				}
				MqttRetainHandling::SendRetainedOnNew => paho_mqtt::RetainHandling::SendRetainedOnNew,
				MqttRetainHandling::DontSendRetained => paho_mqtt::RetainHandling::DontSendRetained,
			});
		}

		options.finalize()
	}
}

struct InnerClient {
	client: paho_mqtt::AsyncClient,
	messages: flume::Receiver<(paho_mqtt::Message, Span)>,
	subscriptions: RefCell<Vec<SubscriptionOptions>>,
}

impl InnerClient {
	fn new(
		client: paho_mqtt::AsyncClient,
		messages: flume::Receiver<(paho_mqtt::Message, Span)>,
	) -> Arc<Self> {
		Self {
			client,
			messages,
			subscriptions: RefCell::default(),
		}
		.into()
	}
}

#[derive(Clone)]
pub struct Client {
	inner: Arc<InnerClient>,
}

impl Client {
	fn client_id(&self) -> String {
		self.inner.client.client_id()
	}

	fn mqtt_version(&self) -> u32 {
		self.inner.client.mqtt_version()
	}
}

#[pin_project]
pub struct MessageStream {
	client_id: String,
	mqtt_version: u32,
	#[pin]
	inner: flume::r#async::RecvStream<'static, (paho_mqtt::Message, Span)>,
}

#[derive(Clone)]
pub struct Message {
	message: paho_mqtt::Message,
}

impl From<paho_mqtt::Message> for Message {
	fn from(message: paho_mqtt::Message) -> Self {
		Self { message }
	}
}

pub struct MessageBuilder {
	builder: paho_mqtt::MessageBuilder,
}

impl MessageBuilder {
	fn new() -> Self {
		Self {
			builder: paho_mqtt::MessageBuilder::new(),
		}
	}
}

impl From<paho_mqtt::MessageBuilder> for MessageBuilder {
	fn from(builder: paho_mqtt::MessageBuilder) -> Self {
		Self { builder }
	}
}

impl Client {
	#[instrument(
		level = Level::DEBUG,
		name = "PahoMqtt::publish",
		skip_all,
		fields(
			client.id = %self.client_id(),
			client.mqtt.version = %self.mqtt_version(),
			message.topic = %builder.message.topic(),
			message.retained = builder.message.retained(),
			message.qos = %builder.message.qos(),
			message.payload.len = builder.message.payload().len(),
		),
		err,
	)]
	async fn publish(&self, builder: PublishBuilder<'_>) -> Result<(), paho_mqtt::Error> {
		self.inner.client.publish(builder.message.message).await
	}

	#[instrument(
		level = Level::DEBUG,
		name = "PahoMqtt::subscribe",
		skip_all,
		fields(
			client.id = %self.client_id(),
			client.mqtt.version = %self.mqtt_version(),
			subscription.topic = %builder.topic,
			subscription.qos = %builder.qos,
		),
		err,
	)]
	async fn subscribe(
		&self,
		builder: SubscribeBuilder<'_>,
	) -> Result<SubscriptionKey, paho_mqtt::Error> {
		let options = SubscriptionOptions::from(builder);
		let key = {
			let subscriptions = self.inner.subscriptions.borrow();
			if subscriptions
				.iter()
				.any(|s| Arc::ptr_eq(&s.topic, &options.topic))
			{
				return Err(paho_mqtt::Error::from(format!(
					"Already subscribed to topic: '{}'",
					options.topic
				)));
			}

			SubscriptionKey {
				key: options.topic.clone(),
			}
		};

		if options.is_empty() {
			self
				.inner
				.client
				.subscribe(options.topic.as_ref(), options.qos.into())
		} else {
			self.inner.client.subscribe_with_options(
				options.topic.as_ref(),
				options.qos.into(),
				paho_mqtt::SubscribeOptions::from(&options),
				None,
			)
		}
		.await
		.map(|_| key)
	}

	#[instrument(
		level = Level::DEBUG,
		name = "PahoMqtt::unsubscribe",
		skip_all,
		fields(
			client.id = %self.client_id(),
			subscription.topic = %builder.key.key,
		),
		err,
	)]
	async fn unsubscribe(&self, builder: UnsubscribeBuilder<'_>) -> Result<(), paho_mqtt::Error> {
		let opts = {
			let mut subscriptions = self.inner.subscriptions.borrow_mut();
			let (idx, _) = subscriptions
				.iter()
				.enumerate()
				.find(|(_, s)| Arc::ptr_eq(&s.topic, &builder.key.key))
				.ok_or_else(|| {
					paho_mqtt::Error::from(format!(
						"Subscription not found for topic: '{}'",
						builder.key.key
					))
				})?;

			subscriptions.swap_remove(idx)
		};

		self
			.inner
			.client
			.unsubscribe(opts.topic.as_ref())
			.await
			.map(|_| ())
	}

	#[instrument(
		level = Level::DEBUG,
		name = "PahoMqtt::disconnect",
		skip_all,
		fields(
			client.id = %self.client_id(),
			client.mqtt.version = %self.mqtt_version(),
			timeout = ?builder.timeout,
			publish_last_will = builder.publish_last_will,
		),
		err,
	)]
	async fn disconnect(&self, builder: DisconnectBuilder<'_>) -> Result<(), paho_mqtt::Error> {
		let mut opts = paho_mqtt::DisconnectOptionsBuilder::new();
		if let Some(timeout) = builder.timeout {
			opts.timeout(timeout);
		}
		if let Some(true) = builder.publish_last_will {
			opts.publish_will_message();
		}

		paho_mqtt::AsyncClient::disconnect(&self.inner.client, opts.finalize())
			.await
			.map(|_| ())
	}
}

impl MqttClient for Client {
	type Provider = PahoMqtt;
	type Message = Message;
	type Messages = MessageStream;
	type SubscriptionKey = SubscriptionKey;
	type PublishBuilder<'a> = PublishBuilder<'a>;
	type SubscribeBuilder<'a> = SubscribeBuilder<'a>;
	type UnsubscribeBuilder<'a> = UnsubscribeBuilder<'a>;
	type DisconnectBuilder<'a> = DisconnectBuilder<'a>;

	fn client_id(&self) -> Arc<str> {
		self.inner.client.client_id().into()
	}

	fn publish(&self, message: Message) -> Self::PublishBuilder<'_> {
		PublishBuilder {
			client: self,
			message,
		}
	}

	fn subscribe(&self, topic: impl Into<Arc<str>>, qos: QosLevel) -> Self::SubscribeBuilder<'_> {
		SubscribeBuilder {
			client: self,
			topic: topic.into(),
			qos,
			no_local: None,
			retain_handling: None,
		}
	}

	fn unsubscribe(&self, key: SubscriptionKey) -> Self::UnsubscribeBuilder<'_> {
		UnsubscribeBuilder { client: self, key }
	}

	fn disconnect(&self) -> Self::DisconnectBuilder<'_> {
		DisconnectBuilder {
			client: self,
			timeout: None,
			publish_last_will: None,
		}
	}

	fn messages(&self) -> Self::Messages {
		MessageStream {
			client_id: self.client_id(),
			mqtt_version: self.mqtt_version(),
			inner: self.inner.messages.clone().into_stream(),
		}
	}
}

pub struct SubscriptionKey {
	// used for pointer equality
	key: Arc<str>,
}

pub struct PublishBuilder<'a> {
	client: &'a Client,
	message: Message,
}

impl<'a> MqttPublishBuilder for PublishBuilder<'a> {
	type Error = paho_mqtt::Error;
}

impl<'a> IntoFuture for PublishBuilder<'a> {
	type Output = Result<(), <Self as MqttPublishBuilder>::Error>;
	type IntoFuture = LocalBoxFuture<'a, Self::Output>;

	fn into_future(self) -> Self::IntoFuture {
		async move { self.client.publish(self).await }.boxed_local()
	}
}

pub struct SubscribeBuilder<'a> {
	client: &'a Client,
	topic: Arc<str>,
	qos: QosLevel,
	no_local: Option<bool>,
	retain_handling: Option<MqttRetainHandling>,
}

impl<'a> MqttSubscribeBuilder for SubscribeBuilder<'a> {
	type Error = paho_mqtt::Error;
	type SubscriptionKey = SubscriptionKey;

	fn no_local(mut self, on: bool) -> Self {
		self.no_local.replace(on);
		self
	}

	fn retain_handling(mut self, handling: MqttRetainHandling) -> Self {
		self.retain_handling.replace(handling);
		self
	}
}

impl<'a> IntoFuture for SubscribeBuilder<'a> {
	type Output = Result<SubscriptionKey, <Self as MqttSubscribeBuilder>::Error>;
	type IntoFuture = LocalBoxFuture<'a, Self::Output>;

	fn into_future(self) -> Self::IntoFuture {
		async move { self.client.subscribe(self).await }.boxed_local()
	}
}

pub struct UnsubscribeBuilder<'a> {
	client: &'a Client,
	key: SubscriptionKey,
}

impl<'a> MqttUnsubscribeBuilder for UnsubscribeBuilder<'a> {
	type Error = paho_mqtt::Error;
}

impl<'a> IntoFuture for UnsubscribeBuilder<'a> {
	type Output = Result<(), <Self as MqttUnsubscribeBuilder>::Error>;
	type IntoFuture = LocalBoxFuture<'a, Self::Output>;

	fn into_future(self) -> Self::IntoFuture {
		async move { self.client.unsubscribe(self).await }.boxed_local()
	}
}

pub struct DisconnectBuilder<'a> {
	client: &'a Client,
	timeout: Option<Duration>,
	publish_last_will: Option<bool>,
}

impl<'a> MqttDisconnectBuilder for DisconnectBuilder<'a> {
	type Error = paho_mqtt::Error;

	fn after(mut self, timeout: Duration) -> Self {
		self.timeout.replace(timeout);
		self
	}

	fn publish_last_will(mut self, publish_last_will: bool) -> Self {
		self.publish_last_will.replace(publish_last_will);
		self
	}
}

impl<'a> IntoFuture for DisconnectBuilder<'a> {
	type Output = Result<(), <Self as MqttDisconnectBuilder>::Error>;
	type IntoFuture = LocalBoxFuture<'a, Self::Output>;

	fn into_future(self) -> Self::IntoFuture {
		async move { self.client.disconnect(self).await }.boxed_local()
	}
}

impl MqttMessage for Message {
	type Client = Client;

	fn topic(&self) -> &str {
		self.message.topic()
	}

	fn payload(&self) -> &[u8] {
		self.message.payload()
	}

	fn retained(&self) -> bool {
		self.message.retained()
	}

	fn qos(&self) -> QosLevel {
		match self.message.qos() {
			paho_mqtt::QOS_0 => QosLevel::AtMostOnce,
			paho_mqtt::QOS_1 => QosLevel::AtLeastOnce,
			paho_mqtt::QOS_2 => QosLevel::ExactlyOnce,
			_ => unreachable!(),
		}
	}
}

impl MqttBuildableMessage for Message {
	type Builder = MessageBuilder;

	fn builder() -> Self::Builder {
		MessageBuilder::new()
	}
}

impl MqttMessageBuilder for MessageBuilder {
	type Message = Message;
	type Error = Infallible;

	fn topic(self, topic: impl Into<String>) -> Self {
		self.builder.topic(topic).into()
	}

	fn payload(self, payload: impl Into<Vec<u8>>) -> Self {
		self.builder.payload(payload).into()
	}

	fn qos(self, qos: crate::QosLevel) -> Self {
		self
			.builder
			.qos(match qos {
				crate::QosLevel::AtMostOnce => paho_mqtt::QOS_0,
				crate::QosLevel::AtLeastOnce => paho_mqtt::QOS_1,
				crate::QosLevel::ExactlyOnce => paho_mqtt::QOS_2,
			})
			.into()
	}

	fn retain(self, retain: bool) -> Self {
		self.builder.retained(retain).into()
	}

	fn build(self) -> Result<Self::Message, Self::Error> {
		Ok(self.builder.finalize().into())
	}
}

impl Stream for MessageStream {
	type Item = MqttReceivedMessage<Client>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		match self.as_mut().project().inner.poll_next(cx) {
			Poll::Ready(Some((message, client_span))) => {
				let span = span!(
					parent: None,
					Level::DEBUG,
					"PahoMqtt::message",
					client.id = %self.client_id,
					client.mqtt.version = %self.mqtt_version,
					message.topic = %message.topic(),
					message.retained = message.retained(),
					message.qos = %message.qos(),
					message.payload.len = message.payload().len(),
				);
				span.follows_from(client_span);
				Poll::Ready(Some(MqttReceivedMessage::new(message.into(), span)))
			}
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Pending => Poll::Pending,
		}
	}
}

impl FusedStream for MessageStream {
	fn is_terminated(&self) -> bool {
		FusedStream::is_terminated(&self.inner)
	}
}

fn as_create_options(
	options: &MqttOptions,
	client_id: &str,
) -> Result<paho_mqtt::CreateOptions, PahoProviderConnectError> {
	let builder = paho_mqtt::CreateOptionsBuilder::new()
		.client_id(client_id)
		.send_while_disconnected(true);

	let builder = builder.persistence(options.persitence.clone());

	Ok(builder.finalize())
}
