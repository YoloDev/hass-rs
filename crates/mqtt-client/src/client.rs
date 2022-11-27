use std::{thread, time::Duration};

use crate::{
	options::HassMqttConnection,
	topics::{DiscoveryTopicConfig, PrivateTopicConfig},
	HassMqttOptions,
};
use error_stack::{report, IntoReport, ResultExt};
use paho_mqtt::AsyncClient as MqttClient;
use thiserror::Error;

enum Command {}

#[derive(Clone, Debug, Error)]
pub enum ConnectError {
	#[error("failed to connect to MQTT broker")]
	Connect,

	#[error("falied to spawn MQTT thread")]
	SpawnThread,

	#[error("failed to create async MQTT runtime")]
	CreateRuntime,
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
	_sender: flume::Sender<Command>,
}

impl HassMqttClient {
	pub async fn new(options: HassMqttOptions) -> error_stack::Result<Self, ConnectError> {
		let sender = Client::spawn(options).await?;
		Ok(Self { _sender: sender })
	}
}

impl HassMqttOptions {
	pub async fn build(self) -> error_stack::Result<HassMqttClient, ConnectError> {
		HassMqttClient::new(self).await
	}
}
