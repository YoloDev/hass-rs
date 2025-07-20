pub(crate) mod command;
pub(crate) mod inner;
pub(crate) mod subscription;

use self::subscription::SubscriptionToken;
use crate::{HassMqttOptions, entity::EntityTopicBuilder};
use futures::Stream;
use hass_dyn_error::DynError;
use hass_mqtt_provider::{MqttProvider, QosLevel};
use pin_project::pin_project;
use std::{
	pin::Pin,
	sync::Arc,
	task::{Context, Poll},
};
use thiserror::Error;
use tracing::{Level, Span, field, instrument, span};

#[derive(Clone)]
pub struct Message {
	pub topic: Arc<str>,
	pub payload: Arc<[u8]>,
	pub retained: bool,
	pub span: Span,
}

impl Message {
	pub fn topic(&self) -> &str {
		&self.topic
	}

	pub fn payload(&self) -> &[u8] {
		&self.payload
	}

	pub fn retained(&self) -> bool {
		self.retained
	}

	pub fn span(&self) -> &Span {
		&self.span
	}
}

#[derive(Clone)]
#[pin_project]
pub(crate) struct Subscription {
	pub(crate) topic: Arc<str>,
	pub(crate) qos: QosLevel,
	pub(crate) token: SubscriptionToken,
	#[pin]
	pub(crate) stream: flume::r#async::RecvStream<'static, Message>,
}

impl Stream for Subscription {
	type Item = Message;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		self.project().stream.poll_next(cx)
	}
}

#[derive(Clone)]
pub struct HassMqttClient {
	client_id: Arc<str>,
	sender: flume::Sender<command::Command>,
}

impl HassMqttClient {
	pub(crate) async fn command<T>(&self, cmd: T) -> command::CommandResult<T>
	where
		T: command::ClientCommand,
		command::Command: command::FromClientCommand<T>,
	{
		let span = Span::current();
		let cmd = Arc::new(cmd);
		let (msg, receiver) = command::Command::from_command(cmd.clone(), span);
		self
			.sender
			.send_async(msg)
			.await
			.map_err(|source| cmd.create_error(source))?;

		match receiver.await {
			Ok(r) => r,
			Err(e) => Err(cmd.create_error(e)),
		}
	}
}

#[derive(Debug, Error)]
#[error("failed to connect to MQTT broker")]
pub struct ConnectError {
	#[cfg_attr(provide_any, backtrace)]
	source: DynError,
}

impl ConnectError {
	pub(crate) fn new(source: impl std::error::Error + Send + Sync + 'static) -> Self {
		Self {
			source: DynError::new(source),
		}
	}
}

impl HassMqttClient {
	#[instrument(
		level = Level::DEBUG,
		name = "HassMqttClient::new",
		skip_all,
		fields(
			provider.name = T::NAME,
		)
		err,
	)]
	pub async fn new<T: MqttProvider>(options: HassMqttOptions) -> Result<Self, ConnectError> {
		let (sender, client_id) = inner::spawn::<T>(options)
			.await
			.map_err(ConnectError::new)?;
		Ok(Self { sender, client_id })
	}
}

impl HassMqttClient {
	pub fn entity(
		&self,
		domain: impl Into<Arc<str>>,
		entity_id: impl Into<Arc<str>>,
	) -> EntityTopicBuilder<'_> {
		self._entity(domain.into(), entity_id.into())
	}

	fn _entity(&self, domain: Arc<str>, entity_id: Arc<str>) -> EntityTopicBuilder<'_> {
		let span = span!(Level::DEBUG, "HassMqttClient::entity", client.id = %self.client_id, entity.domain = %domain, entity.id = %entity_id, entity.topic = field::Empty);
		EntityTopicBuilder::new(self, domain, entity_id, span)
	}
}

#[derive(Debug, Error)]
#[error("failed to publish MQTT message to '{topic}'")]
pub struct PublishMessageError {
	topic: Arc<str>,
	retained: bool,
	qos: QosLevel,
	#[cfg_attr(provide_any, backtrace)]
	source: DynError,
}

impl HassMqttClient {
	#[instrument(
		level = Level::DEBUG,
		name = "HassMqttClient::publish_message",
		skip_all,
		fields(
			client.id = %self.client_id,
			message.topic = %topic,
			message.retained = retained,
			message.qos = %qos,
			message.payload.len = payload.len(),
		))]
	pub(crate) async fn publish_message(
		&self,
		topic: Arc<str>,
		payload: Arc<[u8]>,
		retained: bool,
		qos: QosLevel,
	) -> Result<(), PublishMessageError> {
		self
			.command(command::publish(topic.clone(), payload, retained, qos))
			.await
			.map_err(|source| PublishMessageError {
				topic,
				retained,
				qos,
				source: DynError::new(source),
			})?;

		Ok(())
	}
}

#[derive(Debug, Error)]
#[error("failed to subscribe to MQTT topic '{topic}'")]
pub struct SubscribeError {
	topic: Arc<str>,
	qos: QosLevel,
	#[cfg_attr(provide_any, backtrace)]
	source: DynError,
}

impl HassMqttClient {
	#[instrument(
		level = Level::DEBUG,
		name = "HassMqttClient::subscribe",
		skip_all,
		fields(
			client.id = %self.client_id,
			subscription.topic = %topic,
			subscription.qos,
		))]
	pub(crate) async fn subscribe(
		&self,
		topic: Arc<str>,
		qos: QosLevel,
	) -> Result<Subscription, SubscribeError> {
		let result = self
			.command(command::subscribe(topic.clone(), qos))
			.await
			.map_err(|source| SubscribeError {
				topic: topic.clone(),
				qos,
				source: DynError::new(source),
			})?;

		Ok(Subscription {
			topic,
			qos,
			token: result.token,
			stream: result.receiver.into_stream(),
		})
	}
}

impl HassMqttOptions {
	pub async fn build<T: MqttProvider>(self) -> Result<HassMqttClient, ConnectError> {
		HassMqttClient::new::<T>(self).await
	}

	#[cfg(feature = "paho")]
	#[cfg_attr(doc_cfg, doc(cfg(feature = "paho")))]
	pub async fn build_paho(self) -> Result<HassMqttClient, ConnectError> {
		use hass_mqtt_provider_paho::PahoMqtt;

		self.build::<PahoMqtt>().await
	}
}
