use crate::{
	entity::EntityTopic,
	options::HassMqttConnection,
	topics::{DiscoveryTopicConfig, PrivateTopicConfig},
	HassMqttOptions,
};
use error_stack::{report, IntoReport, ResultExt};
use paho_mqtt::AsyncClient as MqttClient;
use std::{sync::Arc, thread, time::Duration};
use thiserror::Error;
use tokio::sync::oneshot;

#[derive(Clone, Debug, Error)]
pub enum ConnectError {
	#[error("failed to connect to MQTT broker")]
	Connect,

	#[error("falied to spawn MQTT thread")]
	SpawnThread,

	#[error("failed to create async MQTT runtime")]
	CreateRuntime,
}

#[derive(Clone, Debug, Error)]
pub enum ClientError {
	#[error("failed to create entity topic for {domain}.{entity_id}")]
	Entity {
		domain: Arc<str>,
		entity_id: Arc<str>,
	},
}

impl ClientError {
	pub fn entity(domain: Arc<str>, entity_id: Arc<str>) -> Self {
		ClientError::Entity { domain, entity_id }
	}
}

enum Command {
	Entity {
		_domain: Arc<str>,
		_entity_id: Arc<str>,
		_return_channel: oneshot::Sender<EntityCommandResult>,
	},
}

impl Command {
	pub fn entity(
		domain: Arc<str>,
		entity_id: Arc<str>,
	) -> (Self, oneshot::Receiver<EntityCommandResult>) {
		let (return_channel, return_receiver) = oneshot::channel();
		let cmd = Command::Entity {
			_domain: domain,
			_entity_id: entity_id,
			_return_channel: return_channel,
		};
		(cmd, return_receiver)
	}
}

struct EntityCommandResult {
	topic: Arc<str>,
}

struct Client {
	_discovery: DiscoveryTopicConfig,
	_private: PrivateTopicConfig,
	client: MqttClient,
	receiver: flume::Receiver<Command>,
}

impl Client {
	fn new(
		discovery: DiscoveryTopicConfig,
		private: PrivateTopicConfig,
		client: MqttClient,
		receiver: flume::Receiver<Command>,
	) -> Self {
		Client {
			_discovery: discovery,
			_private: private,
			client,
			receiver,
		}
	}

	async fn run(mut self) {
		while let Ok(cmd) = self.receiver.recv_async().await {
			self.handle(cmd).await
		}

		// Try to gracefully exit
		let mut builder = paho_mqtt::DisconnectOptionsBuilder::new();
		builder.timeout(Duration::from_secs(10));
		builder.publish_will_message();
		let _ = self.client.disconnect(builder.finalize()).await;
	}

	async fn handle(&mut self, _cmd: Command) {
		todo!()
	}

	async fn spawn(
		options: HassMqttOptions,
	) -> error_stack::Result<flume::Sender<Command>, ConnectError> {
		let (result_sender, result_receiver) = tokio::sync::oneshot::channel();

		thread::Builder::new()
			.name(format!("mqtt-{}-hass", options.application_slug))
			.spawn(move || {
				let (sender, receiver) = flume::unbounded();
				let rt = match tokio::runtime::Builder::new_current_thread()
					.build()
					.into_report()
					.change_context(ConnectError::CreateRuntime)
				{
					Ok(rt) => rt,
					Err(e) => {
						let _ = result_sender.send(Err(e));
						return;
					}
				};

				let _guard = rt.enter();
				rt.block_on(async move {
					let HassMqttConnection {
						discovery,
						private,
						client: mqtt_client,
					} = match options
						.create_client()
						.await
						.change_context(ConnectError::Connect)
					{
						Ok(c) => c,
						Err(e) => {
							let _ = result_sender.send(Err(e));
							return;
						}
					};

					let client = Client::new(discovery, private, mqtt_client, receiver);

					let _ = result_sender.send(Ok(sender));
					client.run().await;
				});
				todo!();
			})
			.into_report()
			.change_context(ConnectError::SpawnThread)?;

		match result_receiver.await {
			Ok(Ok(sender)) => Ok(sender),
			Ok(Err(e)) => Err(e),
			Err(e) => Err(report!(e).change_context(ConnectError::Connect)),
		}
	}
}

#[derive(Clone)]
pub struct HassMqttClient {
	sender: flume::Sender<Command>,
}

impl HassMqttClient {
	pub async fn new(options: HassMqttOptions) -> error_stack::Result<Self, ConnectError> {
		let sender = Client::spawn(options).await?;
		Ok(Self { sender })
	}

	pub async fn entity(
		&self,
		domain: impl Into<Arc<str>>,
		entity_id: impl Into<Arc<str>>,
	) -> error_stack::Result<EntityTopic, ClientError> {
		let entity_id = entity_id.into();
		let domain = domain.into();

		let (cmd, ret) = Command::entity(domain.clone(), entity_id.clone());

		self
			.sender
			.send_async(cmd)
			.await
			.into_report()
			.change_context_lazy(|| ClientError::entity(domain.clone(), entity_id.clone()))?;

		match ret.await {
			Ok(EntityCommandResult { topic }) => Ok(EntityTopic::new(self.clone(), topic)),
			Err(e) => Err(report!(e).change_context(ClientError::entity(domain, entity_id))),
		}
	}
}

impl HassMqttOptions {
	pub async fn build(self) -> error_stack::Result<HassMqttClient, ConnectError> {
		HassMqttClient::new(self).await
	}
}
