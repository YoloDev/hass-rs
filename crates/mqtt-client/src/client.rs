mod command;
mod inner;
mod subscription;

use self::{inner::InnerClient, subscription::SubscriptionToken};
use crate::{entity::EntityTopic, error::DynError, HassMqttOptions};
use futures::Stream;
use hass_mqtt_provider::{MqttProvider, QosLevel};
use pin_project::pin_project;
use std::{
	pin::Pin,
	sync::Arc,
	task::{Context, Poll},
};
use thiserror::Error;

#[derive(Clone)]
pub struct Message {
	pub topic: Arc<str>,
	pub payload: Arc<[u8]>,
	pub retained: bool,
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
	sender: flume::Sender<command::Command>,
}

impl HassMqttClient {
	async fn command<T>(&self, cmd: T) -> command::CommandResult<T>
	where
		T: command::ClientCommand,
		command::Command: command::FromClientCommand<T>,
	{
		let cmd = Arc::new(cmd);
		let (msg, receiver) = command::Command::from_command(cmd.clone());
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
	pub async fn new<T: MqttProvider>(options: HassMqttOptions) -> Result<Self, ConnectError> {
		let sender = InnerClient::spawn::<T>(options)
			.await
			.map_err(ConnectError::new)?;
		Ok(Self { sender })
	}
}

#[derive(Debug, Error)]
#[error("failed to create MQTT entity: {domain}.{entity_id}")]
pub struct CreateEntityError {
	domain: Arc<str>,
	entity_id: Arc<str>,
	#[cfg_attr(provide_any, backtrace)]
	source: DynError,
}

impl HassMqttClient {
	pub async fn entity(
		&self,
		domain: impl Into<Arc<str>>,
		entity_id: impl Into<Arc<str>>,
	) -> Result<EntityTopic, CreateEntityError> {
		let domain = domain.into();
		let entity_id = entity_id.into();
		let result = self
			.command(command::entity(domain.clone(), entity_id.clone()))
			.await
			.map_err(|source| CreateEntityError {
				domain,
				entity_id,
				source: DynError::new(source),
			})?;

		Ok(EntityTopic::new(self.clone(), result.topics))
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
	pub(crate) async fn publish_message(
		&self,
		topic: impl Into<Arc<str>>,
		payload: impl Into<Arc<[u8]>>,
		retained: bool,
		qos: QosLevel,
	) -> Result<(), PublishMessageError> {
		let topic = topic.into();
		let payload = payload.into();

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
	pub(crate) async fn subscribe(
		&self,
		topic: impl Into<Arc<str>>,
		qos: QosLevel,
	) -> Result<Subscription, SubscribeError> {
		let topic = topic.into();
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
