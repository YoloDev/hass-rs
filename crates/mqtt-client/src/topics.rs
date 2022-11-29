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
		&*self.slug
	}
}

impl fmt::Display for ApplicationName {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Display::fmt(&*self.value, f)
	}
}

#[derive(Clone)]
pub struct DiscoveryTopicConfig {
	_prefix: Arc<str>,
	_node_id: NodeId,
}

impl DiscoveryTopicConfig {
	pub(crate) fn new(prefix: impl Into<Arc<str>>, node_id: NodeId) -> Self {
		DiscoveryTopicConfig {
			_prefix: prefix.into(),
			_node_id: node_id,
		}
	}
}

#[derive(Clone)]
pub struct PrivateTopicConfig {
	prefix: Arc<str>,
	node_id: NodeId,
}

impl PrivateTopicConfig {
	pub(crate) const ONLINE_PLAYLOAD: &'static str = "online";
	pub(crate) const OFFLINE_PLAYLOAD: &'static str = "offline";

	pub(crate) fn new(prefix: impl Into<Arc<str>>, node_id: NodeId) -> Self {
		PrivateTopicConfig {
			prefix: prefix.into(),
			node_id,
		}
	}

	pub(crate) fn available(&self) -> String {
		self.node_topic("available")
	}

	pub(crate) fn node_topic(&self, topic: impl AsRef<str>) -> String {
		format!("{}/{}/{}", self.prefix, self.node_id.0, topic.as_ref())
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
