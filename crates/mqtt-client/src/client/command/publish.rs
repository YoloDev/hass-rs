use super::{Client, ClientCommand};
use crate::{
	provider::{MqttClient, MqttMessage, MqttMessageBuilder},
	MqttQosLevel,
};
use async_trait::async_trait;
use error_stack::ResultExt;
use std::sync::Arc;
use thiserror::Error;

pub(in super::super) struct PublishCommand {
	topic: Arc<str>,
	payload: Arc<[u8]>,
	retained: bool,
	qos: MqttQosLevel,
}

impl PublishCommand {
	pub fn new(topic: Arc<str>, payload: Arc<[u8]>, retained: bool, qos: MqttQosLevel) -> Self {
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
pub struct PublishCommandError {
	topic: Arc<str>,
	retained: bool,
	qos: MqttQosLevel,
}

#[async_trait(?Send)]
impl ClientCommand for PublishCommand {
	type Result = ();
	type Error = PublishCommandError;

	async fn run<T: MqttClient>(
		&self,
		_client: &mut Client,
		mqtt: &T,
	) -> error_stack::Result<Self::Result, Self::Error> {
		let msg = <T::Message as MqttMessage>::builder()
			.topic(&*self.topic)
			.payload(&*self.payload)
			.retain(self.retained)
			.qos(self.qos)
			.build()
			.change_context_lazy(|| self.create_error())?;

		mqtt
			.publish(msg)
			.await
			.change_context_lazy(|| self.create_error())
	}

	fn create_error(&self) -> Self::Error {
		PublishCommandError {
			topic: self.topic.clone(),
			retained: self.retained,
			qos: self.qos,
		}
	}
}
