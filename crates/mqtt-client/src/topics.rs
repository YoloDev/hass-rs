use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct NodeId(Arc<str>);

impl NodeId {
	pub(crate) fn new(value: impl Into<Arc<str>>) -> Self {
		NodeId(value.into())
	}
}

#[derive(Clone)]
pub(crate) struct DiscoveryTopicConfig {
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
pub(crate) struct PrivateTopicConfig {
	prefix: Arc<str>,
	node_id: NodeId,
}

impl PrivateTopicConfig {
	pub(crate) fn new(prefix: impl Into<Arc<str>>, node_id: NodeId) -> Self {
		PrivateTopicConfig {
			prefix: prefix.into(),
			node_id,
		}
	}

	pub(crate) fn node_topic(&self, topic: impl AsRef<str>) -> String {
		format!("{}/{}/{}", self.prefix, self.node_id.0, topic.as_ref())
	}
}
