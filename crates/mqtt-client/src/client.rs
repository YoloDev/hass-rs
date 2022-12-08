mod command;
mod inner;
mod subscription;

use self::{inner::InnerClient, subscription::SubscriptionToken};
use crate::{entity::EntityTopic, mqtt, HassMqttOptions};
use error_stack::{report, IntoReport, ResultExt};
use futures::Stream;
use pin_project::pin_project;
use std::{
	pin::Pin,
	sync::Arc,
	task::{Context, Poll},
};
use thiserror::Error;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum MqttQosLevel {
	AtLeastOnce = 0,
	AtMostOnce = 1,
	ExactlyOnce = 2,
}

impl From<MqttQosLevel> for u8 {
	fn from(qos: MqttQosLevel) -> Self {
		qos as u8
	}
}

impl From<MqttQosLevel> for i32 {
	fn from(qos: MqttQosLevel) -> Self {
		qos as i32
	}
}

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
	pub(crate) qos: MqttQosLevel,
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
			.into_report()
			.change_context_lazy(|| cmd.create_error())?;

		match receiver.await {
			Ok(r) => r,
			Err(e) => Err(report!(e).change_context(cmd.create_error())),
		}
	}
}

#[derive(Debug, Error)]
#[error("failed to connect to MQTT broker")]
pub struct ConnectError {}

impl HassMqttClient {
	pub async fn new<T: mqtt::MqttProvider>(
		options: HassMqttOptions,
	) -> error_stack::Result<Self, ConnectError> {
		let sender = InnerClient::spawn::<T>(options)
			.await
			.change_context_lazy(|| ConnectError {})?;
		Ok(Self { sender })
	}
}

#[derive(Debug, Error)]
#[error("failed to create MQTT entity: {domain}.{entity_id}")]
pub struct CreateEntityError {
	domain: Arc<str>,
	entity_id: Arc<str>,
}

impl HassMqttClient {
	pub async fn entity(
		&self,
		domain: impl Into<Arc<str>>,
		entity_id: impl Into<Arc<str>>,
	) -> error_stack::Result<EntityTopic, CreateEntityError> {
		let domain = domain.into();
		let entity_id = entity_id.into();
		let result = self
			.command(command::entity(domain.clone(), entity_id.clone()))
			.await
			.change_context_lazy(|| CreateEntityError { domain, entity_id })?;

		Ok(EntityTopic::new(self.clone(), result.topics))
	}
}

#[derive(Debug, Error)]
#[error("failed to publish MQTT message to '{topic}'")]
pub struct PublishMessageError {
	topic: Arc<str>,
	retained: bool,
	qos: MqttQosLevel,
}

impl HassMqttClient {
	pub(crate) async fn publish_message(
		&self,
		topic: impl Into<Arc<str>>,
		payload: impl Into<Arc<[u8]>>,
		retained: bool,
		qos: MqttQosLevel,
	) -> error_stack::Result<(), PublishMessageError> {
		let topic = topic.into();
		let payload = payload.into();

		self
			.command(command::publish(topic.clone(), payload, retained, qos))
			.await
			.change_context_lazy(|| PublishMessageError {
				topic,
				retained,
				qos,
			})?;

		Ok(())
	}
}

#[derive(Debug, Error)]
#[error("failed to subscribe to MQTT topic '{topic}'")]
pub struct SubscribeError {
	topic: Arc<str>,
	qos: MqttQosLevel,
}

impl HassMqttClient {
	pub(crate) async fn subscribe(
		&self,
		topic: impl Into<Arc<str>>,
		qos: MqttQosLevel,
	) -> error_stack::Result<Subscription, SubscribeError> {
		let topic = topic.into();
		let result = self
			.command(command::subscribe(topic.clone(), qos))
			.await
			.change_context_lazy(|| SubscribeError {
				topic: topic.clone(),
				qos,
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
	pub async fn build<T: mqtt::MqttProvider>(
		self,
	) -> error_stack::Result<HassMqttClient, ConnectError> {
		HassMqttClient::new::<T>(self).await
	}

	#[cfg(feature = "paho")]
	#[cfg_attr(doc_cfg, doc(cfg(feature = "paho")))]
	pub async fn build_paho(self) -> error_stack::Result<HassMqttClient, ConnectError> {
		self.build::<mqtt::PahoMqtt>().await
	}
}
