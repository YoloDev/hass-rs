use crate::{
	entity::EntityTopic,
	provider::{
		HassMqttConnection, MqttClient, MqttMessage, MqttMessageBuilder, MqttProvider, MqttProviderExt,
	},
	router::Router,
	topics::{EntityTopicsConfig, TopicsConfig},
	HassMqttOptions, MqttQosLevel,
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
	Entity { domain: String, entity_id: String },

	#[error("failed to create MQTT message for topic '{topic}'")]
	CreateMessage { topic: String },

	#[error("failed to publish MQTT message for topic '{topic}'")]
	Publish { topic: String },

	#[error("failed to subscribe to MQTT topic '{topic}'")]
	Subscribe { topic: String },
}

impl ClientError {
	pub fn entity(domain: impl Into<String>, entity_id: impl Into<String>) -> Self {
		let domain = domain.into();
		let entity_id = entity_id.into();
		ClientError::Entity { domain, entity_id }
	}
}

enum Command {
	Entity {
		domain: Arc<str>,
		entity_id: Arc<str>,
		return_channel: oneshot::Sender<EntityCommandResult>,
	},

	Publish {
		topic: String,
		payload: Vec<u8>,
		retained: bool,
		return_channel: oneshot::Sender<error_stack::Result<(), ClientError>>,
	},

	Subscribe {
		topic: String,
		qos: MqttQosLevel,
		return_channel: oneshot::Sender<error_stack::Result<flume::Receiver<Message>, ClientError>>,
	},
}

impl Command {
	pub fn entity(
		domain: Arc<str>,
		entity_id: Arc<str>,
	) -> (Self, oneshot::Receiver<EntityCommandResult>) {
		let (return_channel, return_receiver) = oneshot::channel();
		let cmd = Command::Entity {
			domain,
			entity_id,
			return_channel,
		};
		(cmd, return_receiver)
	}

	pub fn publish(
		topic: String,
		payload: Vec<u8>,
		retained: bool,
	) -> (
		Self,
		oneshot::Receiver<error_stack::Result<(), ClientError>>,
	) {
		let (return_channel, return_receiver) = oneshot::channel();
		let cmd = Command::Publish {
			topic,
			payload,
			retained,
			return_channel,
		};
		(cmd, return_receiver)
	}

	pub fn subscribe(
		topic: String,
		qos: MqttQosLevel,
	) -> (
		Self,
		oneshot::Receiver<error_stack::Result<flume::Receiver<Message>, ClientError>>,
	) {
		let (return_channel, return_receiver) = oneshot::channel();
		let cmd = Command::Subscribe {
			topic,
			qos,
			return_channel,
		};
		(cmd, return_receiver)
	}
}

struct EntityCommandResult {
	topics: EntityTopicsConfig,
}

#[derive(Clone)]
pub struct Message {
	pub topic: Arc<str>,
	pub payload: Arc<[u8]>,
}

struct Client {
	topics: TopicsConfig,
	router: Router<flume::Sender<Message>>,
}

impl Client {
	fn new(topics: TopicsConfig) -> Self {
		Client {
			topics,
			router: Router::new(),
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

	async fn handle_command<T: MqttClient>(&mut self, cmd: Command, client: &T) {
		match cmd {
			Command::Entity {
				domain,
				entity_id,
				return_channel,
			} => {
				let _ = return_channel.send(self.handle_entity_command(client, domain, entity_id).await);
			}

			Command::Publish {
				topic,
				payload,
				retained,
				return_channel,
			} => {
				let _ = return_channel.send(
					self
						.handle_publish_command(client, topic, payload, retained)
						.await,
				);
			}

			Command::Subscribe {
				topic,
				qos,
				return_channel,
			} => {
				let _ = return_channel.send(self.handle_subscribe_command(client, topic, qos).await);
			}
		}
	}

	async fn handle_message<T: MqttClient>(&mut self, msg: T::Message, _client: &T) {
		let topic = msg.topic();
		let handlers = self.router.matches_with_keys(topic);
		if handlers.len() == 0 {
			return;
		}

		let message = Message {
			topic: topic.into(),
			payload: msg.payload().into(),
		};

		let mut to_remove = Vec::new();
		for (key, handler) in handlers {
			if handler.send(message.clone()).is_err() {
				to_remove.push(key);
			}
		}

		for key in to_remove {
			self.router.remove(key);
		}
	}

	async fn handle_entity_command<T: MqttClient>(
		&mut self,
		_client: &T,
		domain: Arc<str>,
		entity_id: Arc<str>,
	) -> EntityCommandResult {
		let topics_config = self.topics.entity(&domain, &entity_id);

		EntityCommandResult {
			topics: topics_config,
		}
	}

	async fn handle_publish_command<T: MqttClient>(
		&mut self,
		client: &T,
		topic: String,
		payload: Vec<u8>,
		retained: bool,
	) -> error_stack::Result<(), ClientError> {
		let msg = <T::Message as MqttMessage>::builder()
			.topic(topic.clone())
			.payload(payload)
			.retain(retained)
			.build()
			.change_context_lazy(|| ClientError::CreateMessage {
				topic: topic.clone(),
			})?;

		client
			.publish(msg)
			.await
			.change_context_lazy(|| ClientError::Publish { topic })
	}

	async fn handle_subscribe_command<T: MqttClient>(
		&mut self,
		client: &T,
		topic: String,
		qos: MqttQosLevel,
	) -> error_stack::Result<flume::Receiver<Message>, ClientError> {
		client
			.subscribe(topic.clone(), qos)
			.await
			.change_context_lazy(|| ClientError::Subscribe {
				topic: topic.clone(),
			})?;

		let (sender, receiver) = flume::unbounded();
		self.router.insert(&topic, sender);
		client
			.subscribe(topic.clone(), qos)
			.await
			.change_context_lazy(|| ClientError::Subscribe {
				topic: topic.clone(),
			})?;

		Ok(receiver)
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
						topics,
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

					let client = Client::new(topics);

					let _ = result_sender.send(Ok(sender));
					client.run(mqtt_client, receiver).await;
				});
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
			.change_context_lazy(|| ClientError::entity(&*domain, &*entity_id))?;

		match ret.await {
			Ok(EntityCommandResult { topics }) => Ok(EntityTopic::new(self.clone(), topics)),
			Err(e) => Err(report!(e).change_context(ClientError::entity(&*domain, &*entity_id))),
		}
	}

	pub(crate) async fn publish(
		&self,
		topic: String,
		payload: impl Into<Vec<u8>>,
		retained: bool,
	) -> error_stack::Result<(), ClientError> {
		let payload = payload.into();

		let (cmd, ret) = Command::publish(topic.clone(), payload, retained);

		self
			.sender
			.send_async(cmd)
			.await
			.into_report()
			.change_context_lazy(|| ClientError::Publish {
				topic: topic.clone(),
			})?;

		match ret.await {
			Ok(Ok(())) => Ok(()),
			Ok(Err(e)) => Err(e),
			Err(e) => Err(report!(e).change_context(ClientError::Publish { topic })),
		}
	}

	pub(crate) async fn subscribe(
		&self,
		topic: String,
		qos: MqttQosLevel,
	) -> error_stack::Result<flume::Receiver<Message>, ClientError> {
		let (cmd, ret) = Command::subscribe(topic.clone(), qos);

		self
			.sender
			.send_async(cmd)
			.await
			.into_report()
			.change_context_lazy(|| ClientError::Subscribe {
				topic: topic.clone(),
			})?;

		match ret.await {
			Ok(Ok(ret)) => Ok(ret),
			Ok(Err(e)) => Err(e),
			Err(e) => Err(report!(e).change_context(ClientError::Subscribe { topic })),
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
