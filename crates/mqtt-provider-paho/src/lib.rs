use async_trait::async_trait;
use futures::{stream::FusedStream, Stream};
use hass_dyn_error::DynError;
use hass_mqtt_provider::{
	AsMqttOptions, MqttClient, MqttMessage, MqttMessageBuilder, MqttOptions, MqttProvider,
	MqttProviderCreateError, QosLevel,
};
use pin_project::pin_project;
use std::{
	convert::Infallible,
	pin::Pin,
	task::{Context, Poll},
	time::Duration,
};
use thiserror::Error;
use tokio::net::lookup_host;

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
	type Client = Client;
	type Message = Message;
	type Error = PahoProviderConnectError;

	async fn create(
		options: &impl AsMqttOptions,
		client_id: &str,
		online_message: Self::Message,
		offline_message: Self::Message,
	) -> Result<Self::Client, Self::Error> {
		let options = options
			.mqtt_options()
			.map_err(|e| PahoProviderConnectError::message("failed to create MQTT options", e))?;

		let mut client = paho_mqtt::AsyncClient::new(as_create_options(&options, client_id)?)
			.map_err(PahoProviderConnectError::client)?;

		let mut builder = paho_mqtt::ConnectOptionsBuilder::new();
		let hosts = lookup_host((&*options.host, options.port))
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

		builder.will_message(offline_message.message);
		client.set_connected_callback(move |c| {
			let client = c.clone();
			let msg = online_message.clone();
			runtime.spawn(async move {
				// TODO: log error
				let _ = client.publish(msg.message).await;
			});
		});

		let messages = MessageStream {
			inner: client.get_stream(100),
		};

		client
			.connect(builder.finalize())
			.await
			.map_err(PahoProviderConnectError::connect)?;

		Ok(Client { client, messages })
	}
}

pub struct Client {
	client: paho_mqtt::AsyncClient,
	messages: MessageStream,
}

#[pin_project]
#[derive(Clone)]
pub struct MessageStream {
	#[pin]
	inner: paho_mqtt::AsyncReceiver<Option<paho_mqtt::Message>>,
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

#[async_trait(?Send)]
impl MqttClient for Client {
	type Message = Message;
	type Messages = MessageStream;
	type PublishError = paho_mqtt::Error;
	type SubscribeError = paho_mqtt::Error;
	type UnsubscribeError = paho_mqtt::Error;
	type DisconnectError = paho_mqtt::Error;

	async fn publish(&self, message: Message) -> Result<(), Self::PublishError> {
		self.client.publish(message.message).await
	}

	async fn subscribe(
		&self,
		topic: impl Into<String>,
		qos: QosLevel,
	) -> Result<(), Self::SubscribeError> {
		self.client.subscribe(topic, qos.into()).await.map(|_| ())
	}

	async fn unsubscribe(&self, topic: impl Into<String>) -> Result<(), Self::UnsubscribeError> {
		self.client.unsubscribe(topic).await.map(|_| ())
	}

	async fn disconnect(
		&self,
		timeout: std::time::Duration,
		publish_last_will: bool,
	) -> Result<(), Self::DisconnectError> {
		let mut builder = paho_mqtt::DisconnectOptionsBuilder::new();
		builder.timeout(timeout);
		if publish_last_will {
			builder.publish_will_message();
		}

		paho_mqtt::AsyncClient::disconnect(&self.client, builder.finalize())
			.await
			.map(|_| ())
	}

	fn messages(&self) -> Self::Messages {
		self.messages.clone()
	}
}

impl MqttMessage for Message {
	type Builder = MessageBuilder;

	fn builder() -> Self::Builder {
		MessageBuilder::new()
	}

	fn topic(&self) -> &str {
		self.message.topic()
	}

	fn payload(&self) -> &[u8] {
		self.message.payload()
	}

	fn retained(&self) -> bool {
		self.message.retained()
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
	type Item = Message;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		loop {
			match self.as_mut().project().inner.poll_next(cx) {
				Poll::Ready(Some(Some(message))) => return Poll::Ready(Some(message.into())),
				Poll::Ready(Some(None)) => continue,
				Poll::Ready(None) => return Poll::Ready(None),
				Poll::Pending => return Poll::Pending,
			}
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
