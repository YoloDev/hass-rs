use std::sync::Arc;

use futures::Stream;
use pin_project::pin_project;

use crate::{
	client::{self, HassMqttClient, PublishCommandError, SubscribeCommandError, Subscription},
	topics::EntityTopicsConfig,
	MqttQosLevel,
};

pub struct EntityTopic {
	client: HassMqttClient,
	topics: EntityTopicsConfig,
}

impl EntityTopic {
	pub(crate) fn new(client: HassMqttClient, topics: EntityTopicsConfig) -> Self {
		EntityTopic { client, topics }
	}

	pub async fn publish(
		&self,
		payload: impl Into<Arc<[u8]>>,
		retained: bool,
		qos: MqttQosLevel,
	) -> error_stack::Result<(), PublishCommandError> {
		let topic = self.topics.discovery_topic();
		self.client.publish(topic, payload, retained, qos).await
	}

	pub fn state_topic(&self, name: &str) -> StateTopic {
		StateTopic::new(self.client.clone(), self.topics.state_topic(name))
	}

	pub async fn command_topic(
		&self,
		name: &str,
		qos: MqttQosLevel,
	) -> error_stack::Result<CommandTopic, SubscribeCommandError> {
		let topic = self.topics.command_topic(name);
		let subscription = self.client.subscribe(topic.clone(), qos).await?;
		Ok(CommandTopic::new(self.client.clone(), subscription))
	}
}

pub struct StateTopic {
	client: HassMqttClient,
	topic: String,
}

impl StateTopic {
	pub(crate) fn new(client: HassMqttClient, topic: String) -> Self {
		StateTopic { client, topic }
	}

	pub async fn publish(
		&self,
		payload: impl Into<Arc<[u8]>>,
		retained: bool,
		qos: MqttQosLevel,
	) -> error_stack::Result<(), PublishCommandError> {
		self
			.client
			.publish(self.topic.clone(), payload, retained, qos)
			.await
	}
}

#[pin_project]
pub struct CommandTopic {
	_client: HassMqttClient,
	#[pin]
	subscription: Subscription,
}

impl CommandTopic {
	pub(crate) fn new(client: HassMqttClient, subscription: Subscription) -> Self {
		CommandTopic {
			_client: client,
			subscription,
		}
	}
}

impl Stream for CommandTopic {
	type Item = Message;

	fn poll_next(
		self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Option<Self::Item>> {
		self
			.project()
			.subscription
			.poll_next(cx)
			.map(|v| v.map(Message::from))
	}
}

pub struct Message {
	topic: Arc<str>,
	payload: Arc<[u8]>,
	retained: bool,
}

impl From<client::Message> for Message {
	fn from(msg: client::Message) -> Self {
		Message {
			topic: msg.topic,
			payload: msg.payload,
			retained: msg.retained,
		}
	}
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
