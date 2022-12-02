use crate::provider::{MqttMessage, MqttMessageBuilder};
use slug::slugify;
use std::{fmt, sync::Arc};

#[derive(Clone)]
pub struct NodeId(Arc<str>);

impl NodeId {
	pub(crate) fn new(value: impl Into<Arc<str>>) -> Self {
		NodeId(value.into())
	}
}

impl fmt::Display for NodeId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Display::fmt(&*self.0, f)
	}
}

#[derive(Clone)]
pub struct ApplicationName {
	value: Arc<str>,
	slug: Arc<str>,
}

impl ApplicationName {
	pub(crate) fn new(value: impl Into<Arc<str>>) -> Self {
		let value = value.into();
		let slug = Arc::from(slugify(&value));
		ApplicationName { value, slug }
	}

	pub(crate) fn slug(&self) -> &str {
		&self.slug
	}
}

impl fmt::Display for ApplicationName {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Display::fmt(&*self.value, f)
	}
}

#[derive(Clone)]
pub struct TopicsConfig {
	private_prefix: Arc<str>,
	discovery_prefix: Arc<str>,
	node_id: NodeId,
}

impl TopicsConfig {
	pub(crate) const ONLINE_PLAYLOAD: &'static str = "online";
	pub(crate) const OFFLINE_PLAYLOAD: &'static str = "offline";

	pub(crate) fn new(
		private_prefix: impl Into<Arc<str>>,
		discovery_prefix: impl Into<Arc<str>>,
		node_id: NodeId,
	) -> Self {
		TopicsConfig {
			private_prefix: private_prefix.into(),
			discovery_prefix: discovery_prefix.into(),
			node_id,
		}
	}

	fn discovery_topic(&self, domain: &str, entity_id: &str) -> String {
		format!(
			"{}/{}/{}/{}/config",
			self.discovery_prefix, domain, self.node_id, entity_id
		)
	}

	pub(crate) fn entity(&self, domain: &str, entity_id: &str) -> EntityTopicsConfig {
		EntityTopicsConfig::new(self, domain, entity_id)
	}

	pub(crate) fn available(&self) -> String {
		self.node_topic("available")
	}

	pub(crate) fn node_topic(&self, topic: impl AsRef<str>) -> String {
		format!(
			"{}/{}/{}",
			self.private_prefix,
			self.node_id,
			topic.as_ref()
		)
	}

	fn entity_topic(&self, domain: &str, entity_id: &str, kind: &str, name: &str) -> String {
		self.node_topic(format!("{}/{}/{}/{}", domain, entity_id, kind, name,))
	}

	fn state_topic(&self, domain: &str, entity_id: &str, name: &str) -> String {
		self.entity_topic(domain, entity_id, "state", name)
	}

	fn command_topic(&self, domain: &str, entity_id: &str, name: &str) -> String {
		self.entity_topic(domain, entity_id, "command", name)
	}

	pub(crate) fn online_message<T: MqttMessage>(
		&self,
	) -> error_stack::Result<T, <<T as MqttMessage>::Builder as MqttMessageBuilder>::Error> {
		availability_message(&self.available(), Self::ONLINE_PLAYLOAD)
	}

	pub(crate) fn offline_message<T: MqttMessage>(
		&self,
	) -> error_stack::Result<T, <<T as MqttMessage>::Builder as MqttMessageBuilder>::Error> {
		availability_message(&self.available(), Self::OFFLINE_PLAYLOAD)
	}
}

pub(crate) struct EntityTopicsConfig {
	topics: TopicsConfig,
	domain: Arc<str>,
	entity_id: Arc<str>,
}

impl EntityTopicsConfig {
	fn new(
		topics: &TopicsConfig,
		domain: impl Into<Arc<str>>,
		entity_id: impl Into<Arc<str>>,
	) -> Self {
		EntityTopicsConfig {
			topics: topics.clone(),
			domain: domain.into(),
			entity_id: entity_id.into(),
		}
	}

	pub(crate) fn discovery_topic(&self) -> String {
		self.topics.discovery_topic(&self.domain, &self.entity_id)
	}

	pub(crate) fn state_topic(&self, name: &str) -> String {
		self.topics.state_topic(&self.domain, &self.entity_id, name)
	}

	pub(crate) fn command_topic(&self, name: &str) -> String {
		self
			.topics
			.command_topic(&self.domain, &self.entity_id, name)
	}
}

fn availability_message<T: MqttMessage>(
	topic: &str,
	content: &str,
) -> error_stack::Result<T, <<T as MqttMessage>::Builder as MqttMessageBuilder>::Error> {
	T::builder()
		.topic(topic)
		.payload(content)
		.qos(crate::MqttQosLevel::ExactlyOnce)
		.retain(true)
		.build()
}
