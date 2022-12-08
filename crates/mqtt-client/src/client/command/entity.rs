use super::{ClientCommand, InnerClient};
use crate::{mqtt::MqttClient, topics::EntityTopicsConfig};
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;

pub(crate) struct EntityCommand {
	domain: Arc<str>,
	entity_id: Arc<str>,
}

impl EntityCommand {
	pub(crate) fn new(domain: Arc<str>, entity_id: Arc<str>) -> Self {
		Self { domain, entity_id }
	}
}

pub(crate) struct EntityCommandResult {
	pub topics: EntityTopicsConfig,
}

#[derive(Debug, Error)]
#[error("failed to create entity topic for {domain}.{entity_id}")]
pub(crate) struct EntityCommandError {
	domain: Arc<str>,
	entity_id: Arc<str>,
}

#[async_trait(?Send)]
impl ClientCommand for EntityCommand {
	type Result = EntityCommandResult;
	type Error = EntityCommandError;

	async fn run<T: MqttClient>(
		&self,
		client: &mut InnerClient,
		_mqtt: &T,
	) -> error_stack::Result<Self::Result, Self::Error> {
		let topics_config = client.topics.entity(&self.domain, &self.entity_id);

		Ok(EntityCommandResult {
			topics: topics_config,
		})
	}

	fn create_error(&self) -> Self::Error {
		EntityCommandError {
			domain: self.domain.clone(),
			entity_id: self.entity_id.clone(),
		}
	}
}
