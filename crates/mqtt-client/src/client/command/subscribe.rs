use super::{ClientCommand, InnerClient};
use crate::{
	client::{subscription::SubscriptionToken, Message, QosLevel},
	router::RouterEntry,
};
use async_trait::async_trait;
use hass_dyn_error::DynError;
use hass_mqtt_provider::MqttClient;
use std::sync::Arc;
use thiserror::Error;

pub(crate) struct SubscribeCommand {
	topic: Arc<str>,
	qos: QosLevel,
}

impl SubscribeCommand {
	pub(crate) fn new(topic: Arc<str>, qos: QosLevel) -> Self {
		SubscribeCommand { topic, qos }
	}
}

pub(crate) struct SubscribeCommandResult {
	pub token: SubscriptionToken,
	pub receiver: flume::Receiver<Message>,
}

#[derive(Debug, Error)]
#[error("failed to subscribe to MQTT topic '{topic}'")]
pub(crate) struct SubscribeCommandError {
	topic: Arc<str>,
	qos: QosLevel,
	#[cfg_attr(provide_any, backtrace)]
	source: DynError,
}

#[async_trait(?Send)]
impl ClientCommand for SubscribeCommand {
	type Result = SubscribeCommandResult;
	type Error = SubscribeCommandError;

	async fn run<T: MqttClient>(
		&self,
		client: &mut InnerClient<T>,
	) -> Result<Self::Result, Self::Error> {
		let (sender, receiver) = flume::unbounded();
		let route_id = match client.router.entry(self.topic.clone()) {
			RouterEntry::Occupied(entry) => entry.insert(sender),
			RouterEntry::Vacant(entry) => {
				let key = client
					.client
					.subscribe(self.topic.clone(), self.qos)
					.await
					.map_err(|source| self.create_error(source))?;

				entry.insert(key, sender)
			}
		};

		let token = client.subscriptions.insert(route_id);
		Ok(SubscribeCommandResult { token, receiver })
	}

	fn create_error(&self, source: impl std::error::Error + Send + Sync + 'static) -> Self::Error {
		SubscribeCommandError {
			topic: self.topic.clone(),
			qos: self.qos,
			source: DynError::new(source),
		}
	}
}
