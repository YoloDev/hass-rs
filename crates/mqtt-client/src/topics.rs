use hass_mqtt_provider::{MqttBuildableMessage, MqttMessageBuilder, QosLevel};
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

impl From<&str> for NodeId {
	fn from(value: &str) -> Self {
		NodeId::new(value)
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

	pub(crate) fn entity(
		&self,
		domain: &str,
		entity_id: &str,
		topic: Option<Arc<str>>,
	) -> EntityTopicsConfig {
		EntityTopicsConfig::new(self, domain, entity_id, topic)
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

	fn entity_topic(&self, domain: &str, entity_id: &str, kind: &str, name: Option<&str>) -> String {
		match name {
			Some(name) => self.node_topic(format!("{domain}/{entity_id}/{kind}/{name}")),
			None => self.node_topic(format!("{domain}/{entity_id}/{kind}")),
		}
	}

	fn state_topic(&self, domain: &str, entity_id: &str, name: Option<&str>) -> String {
		self.entity_topic(domain, entity_id, "state", name)
	}

	fn command_topic(&self, domain: &str, entity_id: &str, name: Option<&str>) -> String {
		self.entity_topic(domain, entity_id, "set", name)
	}

	pub(crate) fn online_message<T: MqttBuildableMessage>(
		&self,
	) -> Result<T, <<T as MqttBuildableMessage>::Builder as MqttMessageBuilder>::Error> {
		availability_message(&self.available(), Self::ONLINE_PLAYLOAD)
	}

	pub(crate) fn offline_message<T: MqttBuildableMessage>(
		&self,
	) -> Result<T, <<T as MqttBuildableMessage>::Builder as MqttMessageBuilder>::Error> {
		availability_message(&self.available(), Self::OFFLINE_PLAYLOAD)
	}
}

pub(crate) struct EntityTopicsConfig {
	topics: TopicsConfig,
	pub(crate) domain: Arc<str>,
	pub(crate) entity_id: Arc<str>,
	pub(crate) discovery_topic: Arc<str>,
}

impl EntityTopicsConfig {
	fn new(
		topics: &TopicsConfig,
		domain: impl Into<Arc<str>>,
		entity_id: impl Into<Arc<str>>,
		topic: Option<Arc<str>>,
	) -> Self {
		let domain = domain.into();
		let entity_id = entity_id.into();
		let discovery_topic =
			topic.unwrap_or_else(|| Arc::from(topics.discovery_topic(&domain, &entity_id)));

		EntityTopicsConfig {
			topics: topics.clone(),
			domain,
			entity_id,
			discovery_topic,
		}
	}

	pub(crate) fn discovery_topic(&self) -> Arc<str> {
		self.discovery_topic.clone()
	}

	pub(crate) fn state_topic(&self, name: Option<&str>) -> String {
		self.topics.state_topic(&self.domain, &self.entity_id, name)
	}

	pub(crate) fn command_topic(&self, name: Option<&str>) -> String {
		self
			.topics
			.command_topic(&self.domain, &self.entity_id, name)
	}
}

fn availability_message<T: MqttBuildableMessage>(
	topic: &str,
	content: &str,
) -> Result<T, <<T as MqttBuildableMessage>::Builder as MqttMessageBuilder>::Error> {
	T::builder()
		.topic(topic)
		.payload(content)
		.qos(QosLevel::ExactlyOnce)
		.retain(true)
		.build()
}
