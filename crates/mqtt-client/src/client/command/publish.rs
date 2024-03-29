use super::{ClientCommand, InnerClient};
use crate::client::QosLevel;
use async_trait::async_trait;
use hass_dyn_error::DynError;
use hass_mqtt_provider::{MqttBuildableMessage, MqttClient, MqttMessageBuilder};
use std::sync::Arc;
use thiserror::Error;

pub(crate) struct PublishCommand {
	topic: Arc<str>,
	payload: Arc<[u8]>,
	retained: bool,
	qos: QosLevel,
}

impl PublishCommand {
	pub fn new(topic: Arc<str>, payload: Arc<[u8]>, retained: bool, qos: QosLevel) -> Self {
		Self {
			topic,
			payload,
			retained,
			qos,
		}
	}
}

#[derive(Debug, Error)]
#[error("failed to publish MQTT message for topic '{topic}'")]
pub(crate) struct PublishCommandError {
	topic: Arc<str>,
	retained: bool,
	qos: QosLevel,
	#[cfg_attr(provide_any, backtrace)]
	source: DynError,
}

#[async_trait(?Send)]
impl ClientCommand for PublishCommand {
	type Result = ();
	type Error = PublishCommandError;

	async fn run<T: MqttClient>(
		&self,
		client: &mut InnerClient<T>,
	) -> Result<Self::Result, Self::Error> {
		let msg = <T::Message as MqttBuildableMessage>::builder()
			.topic(&*self.topic)
			.payload(&*self.payload)
			.retain(self.retained)
			.qos(self.qos)
			.build()
			.map_err(|source| self.create_error(source))?;

		client
			.client
			.publish(msg)
			.await
			.map_err(|source| self.create_error(source))
	}

	fn create_error(&self, source: impl std::error::Error + Send + Sync + 'static) -> Self::Error {
		PublishCommandError {
			topic: self.topic.clone(),
			retained: self.retained,
			qos: self.qos,
			source: DynError::new(source),
		}
	}
}
