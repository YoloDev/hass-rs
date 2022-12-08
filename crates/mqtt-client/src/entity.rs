use crate::{
	client::{HassMqttClient, Message, MqttQosLevel, Subscription},
	topics::EntityTopicsConfig,
};
use error_stack::ResultExt;
use futures::Stream;
use pin_project::pin_project;
use std::sync::Arc;
use thiserror::Error;

pub struct EntityTopic {
	client: HassMqttClient,
	topics: EntityTopicsConfig,
}

impl EntityTopic {
	pub(crate) fn new(client: HassMqttClient, topics: EntityTopicsConfig) -> Self {
		EntityTopic { client, topics }
	}

	pub fn state_topic(&self, name: &str) -> StateTopic {
		StateTopic::new(
			self.client.clone(),
			self.topics.domain.clone(),
			self.topics.entity_id.clone(),
			self.topics.state_topic(name),
		)
	}
}

#[derive(Debug, Error)]
#[error("failed to publish message on behalf of entity {domain}.{entity_id}")]
pub struct EntityPublishError {
	domain: Arc<str>,
	entity_id: Arc<str>,
}

impl EntityTopic {
	pub async fn publish(
		&self,
		payload: impl Into<Arc<[u8]>>,
		retained: bool,
		qos: MqttQosLevel,
	) -> error_stack::Result<(), EntityPublishError> {
		let topic = self.topics.discovery_topic();
		self
			.client
			.publish_message(topic, payload, retained, qos)
			.await
			.change_context_lazy(|| EntityPublishError {
				domain: self.topics.domain.clone(),
				entity_id: self.topics.entity_id.clone(),
			})
	}
}

#[derive(Debug, Error)]
#[error("failed to subscribe to command topic {name} for entity {domain}.{entity_id}")]
pub struct EntitySubscribeError {
	name: Arc<str>,
	domain: Arc<str>,
	entity_id: Arc<str>,
}

impl EntityTopic {
	pub async fn command_topic(
		&self,
		name: &str,
		qos: MqttQosLevel,
	) -> error_stack::Result<CommandTopic, EntitySubscribeError> {
		let topic = self.topics.command_topic(name);
		let subscription = self
			.client
			.subscribe(topic.clone(), qos)
			.await
			.change_context_lazy(|| EntitySubscribeError {
				name: name.into(),
				domain: self.topics.domain.clone(),
				entity_id: self.topics.entity_id.clone(),
			})?;

		Ok(CommandTopic::new(self.client.clone(), subscription))
	}
}

pub struct StateTopic {
	client: HassMqttClient,
	domain: Arc<str>,
	entity_id: Arc<str>,
	topic: String,
}

impl StateTopic {
	pub(crate) fn new(
		client: HassMqttClient,
		domain: Arc<str>,
		entity_id: Arc<str>,
		topic: String,
	) -> Self {
		StateTopic {
			client,
			domain,
			entity_id,
			topic,
		}
	}

	pub async fn publish(
		&self,
		payload: impl Into<Arc<[u8]>>,
		retained: bool,
		qos: MqttQosLevel,
	) -> error_stack::Result<(), EntityPublishError> {
		self
			.client
			.publish_message(self.topic.clone(), payload, retained, qos)
			.await
			.change_context_lazy(|| EntityPublishError {
				domain: self.domain.clone(),
				entity_id: self.entity_id.clone(),
			})
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
		self.project().subscription.poll_next(cx)
	}
}
