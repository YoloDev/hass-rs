use crate::{
	client::{HassMqttClient, Message, Subscription},
	topics::EntityTopicsConfig,
};
use futures::{future::BoxFuture, FutureExt, Stream};
use hass_dyn_error::DynError;
use hass_mqtt_provider::QosLevel;
use pin_project::pin_project;
use std::{
	convert::Infallible,
	future::{self, IntoFuture, Ready},
	sync::Arc,
};
use thiserror::Error;

pub struct EntityTopicBuilder<'a> {
	client: &'a HassMqttClient,
	domain: Arc<str>,
	entity_id: Arc<str>,
	topic: Option<Arc<str>>,
}

impl<'a> EntityTopicBuilder<'a> {
	pub(crate) fn new(
		client: &'a HassMqttClient,
		domain: impl Into<Arc<str>>,
		entity_id: impl Into<Arc<str>>,
	) -> Self {
		EntityTopicBuilder {
			client,
			domain: domain.into(),
			entity_id: entity_id.into(),
			topic: None,
		}
	}

	pub fn with_topic(self, topic: impl Into<Arc<str>>) -> Self {
		EntityTopicBuilder {
			topic: Some(topic.into()),
			..self
		}
	}
}

#[derive(Debug, Error)]
#[error("failed to create MQTT entity: {domain}.{entity_id}")]
pub struct CreateEntityError {
	domain: Arc<str>,
	entity_id: Arc<str>,
	topic: Option<Arc<str>>,
	#[cfg_attr(provide_any, backtrace)]
	source: DynError,
}

impl<'a> IntoFuture for EntityTopicBuilder<'a> {
	type Output = Result<EntityTopic, CreateEntityError>;
	type IntoFuture = BoxFuture<'a, Self::Output>;

	fn into_future(self) -> Self::IntoFuture {
		async move {
			let domain = self.domain;
			let entity_id = self.entity_id;
			let topic = self.topic;

			let result = self
				.client
				.command(crate::client::command::entity(
					domain.clone(),
					entity_id.clone(),
					topic.clone(),
				))
				.await
				.map_err(|source| CreateEntityError {
					domain,
					entity_id,
					topic,
					source: DynError::new(source),
				})?;

			Ok(EntityTopic::new(self.client.clone(), result.topics))
		}
		.boxed()
	}
}

pub struct EntityTopic {
	client: HassMqttClient,
	topics: EntityTopicsConfig,
}

impl EntityTopic {
	pub(crate) fn new(client: HassMqttClient, topics: EntityTopicsConfig) -> Self {
		EntityTopic { client, topics }
	}

	pub fn state_topic(&self) -> StateTopicBuilder {
		StateTopicBuilder {
			entity: self,
			topic: TopicName::Default,
		}
	}
}

#[derive(Debug, Error)]
#[error("failed to publish message on behalf of entity {domain}.{entity_id}")]
pub struct EntityPublishError {
	domain: Arc<str>,
	entity_id: Arc<str>,
	#[cfg_attr(provide_any, backtrace)]
	source: DynError,
}

impl EntityTopic {
	pub async fn publish(
		&self,
		payload: impl Into<Arc<[u8]>>,
		retained: bool,
		qos: QosLevel,
	) -> Result<(), EntityPublishError> {
		let topic = self.topics.discovery_topic();

		self
			.client
			.publish_message(topic, payload, retained, qos)
			.await
			.map_err(|source| EntityPublishError {
				domain: self.topics.domain.clone(),
				entity_id: self.topics.entity_id.clone(),
				source: DynError::new(source),
			})
	}
}

#[derive(Debug, Error)]
#[error("failed to subscribe to command topic '{topic}' for entity {domain}.{entity_id}")]
pub struct EntitySubscribeError {
	domain: Arc<str>,
	entity_id: Arc<str>,
	topic: Arc<str>,
	#[cfg_attr(provide_any, backtrace)]
	source: DynError,
}

impl EntityTopic {
	pub fn command_topic(&self) -> CommandTopicBuilder {
		CommandTopicBuilder {
			entity: self,
			topic: TopicName::Default,
			qos: QosLevel::AtMostOnce,
		}
	}
}

enum TopicName {
	Default,
	Named(String),
	Custom(Arc<str>),
}

impl TopicName {
	pub fn get(self, f: impl FnOnce(Option<&str>) -> String) -> Arc<str> {
		match self {
			TopicName::Default => Arc::from(f(None)),
			TopicName::Named(name) => Arc::from(f(Some(&name))),
			TopicName::Custom(topic) => topic,
		}
	}
}

pub struct StateTopicBuilder<'a> {
	entity: &'a EntityTopic,
	topic: TopicName,
}

impl<'a> StateTopicBuilder<'a> {
	pub fn name(self, name: impl Into<String>) -> Self {
		StateTopicBuilder {
			topic: TopicName::Named(name.into()),
			..self
		}
	}

	pub fn topic(self, topic: impl Into<Arc<str>>) -> Self {
		StateTopicBuilder {
			topic: TopicName::Custom(topic.into()),
			..self
		}
	}
}

impl<'a> IntoFuture for StateTopicBuilder<'a> {
	type Output = Result<StateTopic, Infallible>;
	type IntoFuture = Ready<Self::Output>;

	fn into_future(self) -> Self::IntoFuture {
		let topic = self.topic.get(|s| self.entity.topics.state_topic(s));
		future::ready(Ok(StateTopic::new(
			self.entity.client.clone(),
			self.entity.topics.domain.clone(),
			self.entity.topics.domain.clone(),
			topic,
		)))
	}
}

pub struct CommandTopicBuilder<'a> {
	entity: &'a EntityTopic,
	topic: TopicName,
	qos: QosLevel,
}

impl<'a> CommandTopicBuilder<'a> {
	pub fn name(self, name: impl Into<String>) -> Self {
		CommandTopicBuilder {
			topic: TopicName::Named(name.into()),
			..self
		}
	}

	pub fn topic(self, topic: impl Into<Arc<str>>) -> Self {
		CommandTopicBuilder {
			topic: TopicName::Custom(topic.into()),
			..self
		}
	}

	pub fn qos(self, qos: QosLevel) -> Self {
		CommandTopicBuilder { qos, ..self }
	}
}

impl<'a> IntoFuture for CommandTopicBuilder<'a> {
	type Output = Result<CommandTopic, EntitySubscribeError>;
	type IntoFuture = BoxFuture<'a, Self::Output>;

	fn into_future(self) -> Self::IntoFuture {
		async move {
			let topic = self.topic.get(|s| self.entity.topics.command_topic(s));
			let subscription = self
				.entity
				.client
				.subscribe(topic.clone(), self.qos)
				.await
				.map_err(|source| EntitySubscribeError {
					domain: self.entity.topics.domain.clone(),
					entity_id: self.entity.topics.entity_id.clone(),
					topic: topic.clone(),
					source: DynError::new(source),
				})?;

			Ok(CommandTopic::new(self.entity.client.clone(), subscription))
		}
		.boxed()
	}
}

pub struct StateTopic {
	client: HassMqttClient,
	domain: Arc<str>,
	entity_id: Arc<str>,
	topic: Arc<str>,
}

impl<'a> From<&'a StateTopic> for hass_mqtt_types::Topic<'a> {
	fn from(topic: &'a StateTopic) -> Self {
		topic.topic.as_ref().into()
	}
}

impl StateTopic {
	pub(crate) fn new(
		client: HassMqttClient,
		domain: Arc<str>,
		entity_id: Arc<str>,
		topic: Arc<str>,
	) -> Self {
		StateTopic {
			client,
			domain,
			entity_id,
			topic,
		}
	}

	pub fn topic(&self) -> Arc<str> {
		self.topic.clone()
	}

	pub async fn publish(
		&self,
		payload: impl Into<Arc<[u8]>>,
		retained: bool,
		qos: QosLevel,
	) -> Result<(), EntityPublishError> {
		self
			.client
			.publish_message(self.topic.clone(), payload, retained, qos)
			.await
			.map_err(|source| EntityPublishError {
				domain: self.domain.clone(),
				entity_id: self.entity_id.clone(),
				source: DynError::new(source),
			})
	}
}

#[pin_project]
pub struct CommandTopic {
	_client: HassMqttClient,
	#[pin]
	subscription: Subscription,
}

impl<'a> From<&'a CommandTopic> for hass_mqtt_types::Topic<'a> {
	fn from(topic: &'a CommandTopic) -> Self {
		topic.subscription.topic.as_ref().into()
	}
}

impl CommandTopic {
	pub(crate) fn new(client: HassMqttClient, subscription: Subscription) -> Self {
		CommandTopic {
			_client: client,
			subscription,
		}
	}

	pub fn topic(&self) -> Arc<str> {
		self.subscription.topic.clone()
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
