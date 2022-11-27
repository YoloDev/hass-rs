use crate::client::HassMqttClient;
use std::sync::Arc;

pub struct EntityTopic {
	_client: HassMqttClient,
	_topic: Arc<str>,
}

impl EntityTopic {
	pub fn new(client: HassMqttClient, topic: Arc<str>) -> Self {
		EntityTopic {
			_client: client,
			_topic: topic,
		}
	}
}
