use super::{Client, ClientCommand};
use crate::{
	client::{subscription::SubscriptionToken, Message},
	provider::MqttClient,
	MqttQosLevel,
};
use async_trait::async_trait;
use error_stack::ResultExt;
use std::sync::Arc;
use thiserror::Error;

pub(in super::super) struct SubscribeCommand {
	topic: Arc<str>,
	qos: MqttQosLevel,
}

impl SubscribeCommand {
	pub(in super::super) fn new(topic: Arc<str>, qos: MqttQosLevel) -> Self {
		SubscribeCommand { topic, qos }
	}
}

pub(in super::super) struct SubscribeCommandResult {
	pub token: SubscriptionToken,
	pub receiver: flume::Receiver<Message>,
}

#[derive(Debug, Error)]
#[error("failed to subscribe to MQTT topic '{topic}'")]
pub struct SubscribeCommandError {
	topic: Arc<str>,
	qos: MqttQosLevel,
}

#[async_trait(?Send)]
impl ClientCommand for SubscribeCommand {
	type Result = SubscribeCommandResult;
	type Error = SubscribeCommandError;

	async fn run<T: MqttClient>(
		&self,
		client: &mut Client,
		mqtt: &T,
	) -> error_stack::Result<Self::Result, Self::Error> {
		let (sender, receiver) = flume::unbounded();
		let route_id = client.router.insert(&self.topic, sender);
		let token = client.subscriptions.insert(route_id);

		// Note: if the subscription fails, the token gets dropped,
		// which in turn will clean up the route in the router.
		mqtt
			.subscribe(&*self.topic, self.qos)
			.await
			.change_context_lazy(|| self.create_error())?;

		Ok(SubscribeCommandResult { token, receiver })
	}

	fn create_error(&self) -> Self::Error {
		SubscribeCommandError {
			topic: self.topic.clone(),
			qos: self.qos,
		}
	}
}
