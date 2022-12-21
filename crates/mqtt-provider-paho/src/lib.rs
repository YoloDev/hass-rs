use async_trait::async_trait;
use futures::{stream::FusedStream, Stream};
use hass_dyn_error::DynError;
use hass_mqtt_provider::{
	AsMqttOptions, MqttBuildableMessage, MqttClient, MqttMessage, MqttMessageBuilder, MqttOptions,
	MqttProvider, MqttProviderCreateError, MqttReceivedMessage, QosLevel,
};
use pin_project::pin_project;
use std::{
	convert::Infallible,
	pin::Pin,
	sync::Arc,
	task::{Context, Poll},
	time::Duration,
};
use thiserror::Error;
use tokio::net::lookup_host;
use tracing::{event, instrument, span, Instrument, Level, Span};

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
			.automatic_reconnect(Duration::from_secs(5), Duration::from_secs(60 * 5));

		#[cfg(feature = "tls")]
		if options.tls {
			builder.ssl_options(paho_mqtt::SslOptions::new());
		}

		if let Some(auth) = &options.auth {
			builder.user_name(auth.username.clone());
			builder.password(auth.password.clone());
		}

		let runtime = tokio::runtime::Handle::current();
		let span = Span::current();

		let (message_sender, message_receiver) = flume::unbounded();

		builder.will_message(offline_message.message);
		client.set_connected_callback({
			let span = span.clone();
			move |c| {
				let client = c.clone();
				let msg = online_message.clone();
				let span = span.clone();
				let client_id = c.client_id();
				runtime.spawn(async move {
					// TODO: (Re)subscribe to topics
					if let Err(e) = client.publish(msg.message).await {
						event!(
							parent: &span,
							Level::ERROR,
							client.id = %client_id,
							"failed to publish online message: {:#}",
							e,
						);
					}
				});
			}
		});

		client.set_connection_lost_callback({
			let span = span.clone();
			move |c| {
				event!(
				parent: &span,
				Level::WARN,
				client.id = %c.client_id(),
				"connection lost");
			}
		});

		client.set_disconnected_callback({
			let span = span.clone();
			move |c, _props, reason| {
				event!(
				parent: &span,
				Level::INFO,
				client.id = %c.client_id(),
				reason = %reason,
				"disconnected");
			}
		});

		client.set_message_callback({
			let span = span.clone();
			move |c, message| {
				if let Some(message) = message {
					event!(
						parent: &span,
						Level::DEBUG,
						client.id = %c.client_id(),
						message.topic = %message.topic(),
						message.retained = message.retained(),
						message.qos = %message.qos(),
						message.payload.len = message.payload().len(),
					);
					if let Err(e) = message_sender.send((message, span.clone())) {
						event!(
							parent: &span,
							Level::ERROR,
							client.id = %c.client_id(),
							"failed to send message to listeners: {:#}",
							e,
						);
					}
				}
			}
		});

		client
			.connect(builder.finalize())
			.instrument(span!(Level::DEBUG, "PahoMqtt::connect", client.id = %client_id))
			.await
			.map_err(PahoProviderConnectError::connect)?;

		Ok(Client {
			client,
			messages: message_receiver,
		})
	}
}

pub struct Client {
	client: paho_mqtt::AsyncClient,
	messages: flume::Receiver<(paho_mqtt::Message, Span)>,
}

impl Client {
	fn client_id(&self) -> String {
		self.client.client_id()
	}
}

#[pin_project]
pub struct MessageStream {
	client_id: String,
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

// impl ReceivedMessage {
// 	fn new(message: paho_mqtt::Message, client: &paho_mqtt::AsyncClient) -> Self {
// 		let span = span!(
// 			Level::DEBUG,
// 			"PahoMqtt::recv_message",
// 			client.id = %client.client_id(),
// 			message.topic = %message.topic(),
// 			message.retained = message.retained(),
// 			message.qos = %message.qos(),
// 			message.payload.len = message.payload().len(),
// 		);
// 		Self { message, span }
// 	}
// }

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
			message.topic = %message.topic(),
			message.retained = message.retained(),
			message.qos = %message.qos(),
			message.payload.len = message.payload().len(),
		),
		err,
	)]
	async fn publish(&self, message: Message) -> Result<(), paho_mqtt::Error> {
		self.client.publish(message.message).await
	}

	#[instrument(
		level = Level::DEBUG,
		name = "PahoMqtt::subscribe",
		skip_all,
		fields(
			client.id = %self.client_id(),
			subscription.topic = %topic,
			subscription.qos = %qos,
		),
		err,
	)]
	async fn subscribe(&self, topic: String, qos: QosLevel) -> Result<(), paho_mqtt::Error> {
		self.client.subscribe(topic, qos.into()).await.map(|_| ())
	}

	#[instrument(
		level = Level::DEBUG,
		name = "PahoMqtt::unsubscribe",
		skip_all,
		fields(
			client.id = %self.client_id(),
			subscription.topic = %topic,
		),
		err,
	)]
	async fn unsubscribe(&self, topic: String) -> Result<(), paho_mqtt::Error> {
		self.client.unsubscribe(topic).await.map(|_| ())
	}

	#[instrument(
		level = Level::DEBUG,
		name = "PahoMqtt::disconnect",
		skip_all,
		fields(
			client.id = %self.client_id(),
			timeout = ?timeout,
			publish_last_will = publish_last_will,
		),
		err,
	)]
	async fn disconnect(
		&self,
		timeout: std::time::Duration,
		publish_last_will: bool,
	) -> Result<(), paho_mqtt::Error> {
		let mut builder = paho_mqtt::DisconnectOptionsBuilder::new();
		builder.timeout(timeout);
		if publish_last_will {
			builder.publish_will_message();
		}

		paho_mqtt::AsyncClient::disconnect(&self.client, builder.finalize())
			.await
			.map(|_| ())
	}
}

#[async_trait(?Send)]
impl MqttClient for Client {
	type Provider = PahoMqtt;
	type Message = Message;
	type Messages = MessageStream;
	type PublishError = paho_mqtt::Error;
	type SubscribeError = paho_mqtt::Error;
	type UnsubscribeError = paho_mqtt::Error;
	type DisconnectError = paho_mqtt::Error;

	fn client_id(&self) -> Arc<str> {
		self.client.client_id().into()
	}

	async fn publish(&self, message: Message) -> Result<(), Self::PublishError> {
		self.publish(message).await
	}

	async fn subscribe(
		&self,
		topic: impl Into<String>,
		qos: QosLevel,
	) -> Result<(), Self::SubscribeError> {
		self.subscribe(topic.into(), qos).await
	}

	async fn unsubscribe(&self, topic: impl Into<String>) -> Result<(), Self::UnsubscribeError> {
		self.unsubscribe(topic.into()).await
	}

	async fn disconnect(
		&self,
		timeout: std::time::Duration,
		publish_last_will: bool,
	) -> Result<(), Self::DisconnectError> {
		self.disconnect(timeout, publish_last_will).await
	}

	fn messages(&self) -> Self::Messages {
		MessageStream {
			client_id: self.client_id(),
			inner: self.messages.clone().into_stream(),
		}
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
