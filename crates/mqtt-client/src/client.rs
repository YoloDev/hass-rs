use crate::{
	entity::EntityTopic,
	provider::{HassMqttConnection, MqttClient, MqttProvider, MqttProviderExt},
	topics::{DiscoveryTopicConfig, PrivateTopicConfig},
	HassMqttOptions,
};
use error_stack::{report, IntoReport, ResultExt};
use futures::{pin_mut, stream::Fuse, Stream, StreamExt};
use pin_project::pin_project;
use std::{sync::Arc, task::Poll, thread, time::Duration};
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
}

impl Client {
	fn new(discovery: DiscoveryTopicConfig, private: PrivateTopicConfig) -> Self {
		Client {
			_discovery: discovery,
			_private: private,
		}
	}

	async fn run<T: MqttClient>(mut self, client: T, receiver: flume::Receiver<Command>) {
		let events = Events::new(&receiver, &client).fuse();
		pin_mut!(events);

		while let Some(evt) = events.next().await {
			match evt {
				Event::Command(cmd) => self.handle_command(cmd, &client).await,
				Event::Message(msg) => self.handle_message(msg, &client).await,
			}
		}

		let _ = client.disconnect(Duration::from_secs(10), true).await;
	}

	async fn handle_command<T: MqttClient>(&mut self, _cmd: Command, _client: &T) {
		todo!()
	}

	async fn handle_message<T: MqttClient>(&mut self, _msg: T::Message, _client: &T) {
		todo!()
	}

	async fn spawn<P: MqttProvider>(
		options: HassMqttOptions,
	) -> error_stack::Result<flume::Sender<Command>, ConnectError> {
		let (result_sender, result_receiver) = tokio::sync::oneshot::channel();

		thread::Builder::new()
			.name(format!("mqtt-{}-hass", options.application_name.slug()))
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
					} = match <P as MqttProviderExt>::create_client(&options)
						.await
						.change_context(ConnectError::Connect)
					{
						Ok(c) => c,
						Err(e) => {
							let _ = result_sender.send(Err(e));
							return;
						}
					};

					let client = Client::new(discovery, private);

					let _ = result_sender.send(Ok(sender));
					client.run(mqtt_client, receiver).await;
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
	pub async fn new<T: MqttProvider>(
		options: HassMqttOptions,
	) -> error_stack::Result<Self, ConnectError> {
		let sender = Client::spawn::<T>(options).await?;
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
	pub async fn build<T: MqttProvider>(self) -> error_stack::Result<HassMqttClient, ConnectError> {
		HassMqttClient::new::<T>(self).await
	}
}

enum Event<T: MqttClient> {
	Command(Command),
	Message(T::Message),
}

#[pin_project]
struct Events<'a, T: MqttClient> {
	#[pin]
	command_stream: Fuse<flume::r#async::RecvStream<'a, Command>>,

	#[pin]
	message_stream: Fuse<T::Messages>,
}

impl<'a, T: MqttClient> Events<'a, T> {
	fn new(commands: &'a flume::Receiver<Command>, client: &T) -> Self {
		Self {
			command_stream: commands.stream().fuse(),
			message_stream: client.messages().fuse(),
		}
	}
}

impl<'a, T: MqttClient> Stream for Events<'a, T> {
	type Item = Event<T>;

	fn poll_next(
		self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Option<Self::Item>> {
		let this = self.project();

		match this.message_stream.poll_next(cx) {
			Poll::Ready(Some(msg)) => Poll::Ready(Some(Event::Message(msg))),
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Pending => match this.command_stream.poll_next(cx) {
				Poll::Ready(Some(cmd)) => Poll::Ready(Some(Event::Command(cmd))),
				Poll::Ready(None) => Poll::Ready(None),
				Poll::Pending => Poll::Pending,
			},
		}
	}
}
