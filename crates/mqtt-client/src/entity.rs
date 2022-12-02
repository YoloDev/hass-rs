use futures::Stream;
use pin_project::pin_project;

use crate::{
	client::{ClientError, HassMqttClient, Message},
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
		payload: impl Into<Vec<u8>>,
		retained: bool,
	) -> error_stack::Result<(), ClientError> {
		let topic = self.topics.discovery_topic();
		self.client.publish(topic, payload, retained).await
	}

	pub fn state_topic(&self, name: &str) -> StateTopic {
		StateTopic::new(self.client.clone(), self.topics.state_topic(name))
	}

	pub async fn command_topic(
		&self,
		name: &str,
		qos: MqttQosLevel,
	) -> error_stack::Result<CommandTopic, ClientError> {
		let topic = self.topics.command_topic(name);
		let receiver = self.client.subscribe(topic.clone(), qos).await?;
		Ok(CommandTopic::new(self.client.clone(), topic, receiver))
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
		payload: impl Into<Vec<u8>>,
		retained: bool,
	) -> error_stack::Result<(), ClientError> {
		self
			.client
			.publish(self.topic.clone(), payload, retained)
			.await
	}
}

#[pin_project]
pub struct CommandTopic {
	_client: HassMqttClient,
	_topic: String,
	#[pin]
	channel: flume::r#async::RecvStream<'static, Message>,
}

impl CommandTopic {
	pub(crate) fn new(
		client: HassMqttClient,
		topic: String,
		channel: flume::Receiver<Message>,
	) -> Self {
		CommandTopic {
			_client: client,
			_topic: topic,
			channel: channel.into_stream(),
		}
	}
}

impl Stream for CommandTopic {
	type Item = Message;

	fn poll_next(
		self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Option<Self::Item>> {
		self.project().channel.poll_next(cx)
	}
}
