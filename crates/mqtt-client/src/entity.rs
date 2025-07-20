use crate::{
	client::{HassMqttClient, Message, Subscription},
	topics::EntityTopicsConfig,
};
use futures::{FutureExt, Stream, future::BoxFuture};
use hass_dyn_error::DynError;
use hass_mqtt_provider::QosLevel;
use opentelemetry::trace::{SpanContext, TraceContextExt};
use pin_project::pin_project;
use std::{
	convert::Infallible,
	future::{self, IntoFuture},
	sync::Arc,
};
use thiserror::Error;
use tracing::{Instrument, Level, Span, instrument, span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

pub struct EntityTopicBuilder<'a> {
	client: &'a HassMqttClient,
	domain: Arc<str>,
	entity_id: Arc<str>,
	topic: Option<Arc<str>>,
	span: Span,
}

impl<'a> EntityTopicBuilder<'a> {
	pub(crate) fn new(
		client: &'a HassMqttClient,
		domain: Arc<str>,
		entity_id: Arc<str>,
		span: Span,
	) -> Self {
		EntityTopicBuilder {
			client,
			domain,
			entity_id,
			topic: None,
			span,
		}
	}

	pub fn with_topic(self, topic: impl Into<Arc<str>>) -> Self {
		let topic = topic.into();
		self
			.span
			.record("entity.topic", tracing::field::display(&topic));

		EntityTopicBuilder {
			topic: Some(topic),
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
		let EntityTopicBuilder {
			client,
			domain,
			entity_id,
			topic,
			span,
		} = self;

		let span_context = span.context().span().span_context().clone();
		async move {
			let result = client
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

			Ok(EntityTopic::new(
				client.clone(),
				result.topics,
				span_context,
			))
		}
		.instrument(span)
		.boxed()
	}
}

pub struct EntityTopic {
	client: HassMqttClient,
	topics: EntityTopicsConfig,
	span_context: SpanContext,
}

impl EntityTopic {
	pub(crate) fn new(
		client: HassMqttClient,
		topics: EntityTopicsConfig,
		span_context: SpanContext,
	) -> Self {
		EntityTopic {
			client,
			topics,
			span_context,
		}
	}

	pub fn state_topic(&self) -> StateTopicBuilder<'_> {
		let span = span!(
			Level::DEBUG,
			"EntityTopic::state_topic",
			entity.domain = %self.topics.domain,
			entity.entity_id = %self.topics.entity_id);

		StateTopicBuilder {
			entity: self,
			topic: TopicName::Default,
			span,
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
		self._publish(payload.into(), retained, qos).await
	}

	#[instrument(
		level = Level::DEBUG,
		name = "EntityTopic::publish",
		skip_all,
		fields(
			entity.topic,
			message.retained = retained,
			message.qos = %qos,
			message.payload.len = payload.len(),
		)
	)]
	async fn _publish(
		&self,
		payload: Arc<[u8]>,
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
	pub fn command_topic(&self) -> CommandTopicBuilder<'_> {
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
	span: Span,
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
	type IntoFuture = BoxFuture<'a, Self::Output>;

	fn into_future(self) -> Self::IntoFuture {
		let StateTopicBuilder {
			entity,
			topic,
			span,
		} = self;
		let span_context = span.context().span().span_context().clone();

		let topic = topic.get(|s| self.entity.topics.state_topic(s));
		future::ready(Ok(StateTopic::new(
			entity.client.clone(),
			entity.topics.domain.clone(),
			entity.topics.domain.clone(),
			topic,
			span_context,
		)))
		.instrument(span)
		.boxed()
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
		let topic = self.topic.get(|s| self.entity.topics.command_topic(s));
		let span = tracing::info_span!(
			"EntityTopic::command_topic",
			entity = %self.entity.topics.entity_id,
			topic = %topic,
		);
		span.add_link(self.entity.span_context.clone());

		let span_context = span.context().span().span_context().clone();

		async move {
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

			Ok(CommandTopic::new(
				self.entity.client.clone(),
				subscription,
				span_context,
			))
		}
		.instrument(span)
		.boxed()
	}
}

pub struct StateTopic {
	client: HassMqttClient,
	domain: Arc<str>,
	entity_id: Arc<str>,
	topic: Arc<str>,
	span_context: SpanContext,
}

impl<'a> From<&'a StateTopic> for hass_mqtt_proto::Topic<'a> {
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
		span_context: SpanContext,
	) -> Self {
		StateTopic {
			client,
			domain,
			entity_id,
			topic,
			span_context,
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
		self._publish(payload.into(), retained, qos).await
	}

	#[instrument(
		level = Level::DEBUG,
		name = "StateTopic::publish",
		skip_all,
		fields(
			state.topic = %self.topic,
			message.retained = retained,
			message.qos = %qos,
			message.payload.len = payload.len(),
		),
	)]
	async fn _publish(
		&self,
		payload: Arc<[u8]>,
		retained: bool,
		qos: QosLevel,
	) -> Result<(), EntityPublishError> {
		Span::current().add_link(self.span_context.clone());

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
	span_context: SpanContext,
}

impl<'a> From<&'a CommandTopic> for hass_mqtt_proto::Topic<'a> {
	fn from(topic: &'a CommandTopic) -> Self {
		topic.subscription.topic.as_ref().into()
	}
}

impl CommandTopic {
	pub(crate) fn new(
		client: HassMqttClient,
		subscription: Subscription,
		span_context: SpanContext,
	) -> Self {
		CommandTopic {
			_client: client,
			subscription,
			span_context,
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
