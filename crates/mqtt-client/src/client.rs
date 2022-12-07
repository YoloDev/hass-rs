mod command;
mod subscription;

use self::{
	command::{
		ClientCommand, Command, CommandResult, EntityCommand, FromClientCommand, PublishCommand,
		SubscribeCommand,
	},
	subscription::{SubscriptionToken, Subscriptions},
};
use crate::{
	entity::EntityTopic,
	provider::{HassMqttConnection, MqttClient, MqttMessage, MqttProvider, MqttProviderExt},
	router::Router,
	topics::TopicsConfig,
	HassMqttOptions, MqttQosLevel,
};
use error_stack::{report, IntoReport, ResultExt};
use futures::{pin_mut, stream::Fuse, Stream, StreamExt};
use pin_project::pin_project;
use std::{
	pin::Pin,
	sync::Arc,
	task::{Context, Poll},
	thread,
	time::Duration,
};
use thiserror::Error;

// TODO: These should probably be private and wrapped
pub use command::{EntityCommandError, PublishCommandError, SubscribeCommandError};

#[derive(Clone, Debug, Error)]
pub enum ConnectError {
	#[error("failed to connect to MQTT broker")]
	Connect,

	#[error("falied to spawn MQTT thread")]
	SpawnThread,

	#[error("failed to create async MQTT runtime")]
	CreateRuntime,
}

#[derive(Clone)]
pub(crate) struct Message {
	pub topic: Arc<str>,
	pub payload: Arc<[u8]>,
	pub retained: bool,
}

#[derive(Clone)]
#[pin_project]
pub(crate) struct Subscription {
	pub topic: Arc<str>,
	pub qos: MqttQosLevel,
	pub token: SubscriptionToken,
	#[pin]
	pub stream: flume::r#async::RecvStream<'static, Message>,
}

impl Stream for Subscription {
	type Item = Message;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		self.project().stream.poll_next(cx)
	}
}

struct Client {
	topics: TopicsConfig,
	router: Router<flume::Sender<Message>>,
	subscriptions: Subscriptions,
}

impl Client {
	fn new(topics: TopicsConfig) -> Self {
		Client {
			topics,
			router: Router::new(),
			subscriptions: Subscriptions::new(),
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
		cmd.run(self, client).await
	}

	async fn handle_message<T: MqttClient>(&mut self, msg: T::Message, _client: &T) {
		let topic = msg.topic();
		let matches = self.router.matches(topic);
		if matches.len() == 0 {
			return;
		}

		let message = Message {
			topic: topic.into(),
			payload: msg.payload().into(),
			retained: msg.retained(),
		};

		let mut to_remove = Vec::new();
		for handler in matches {
			if handler.send(message.clone()).is_err() {
				to_remove.push(handler.id());
			}
		}

		for key in to_remove {
			self.router.remove(key);
		}
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

	async fn command<T>(&self, cmd: T) -> CommandResult<T>
	where
		T: ClientCommand,
		Command: FromClientCommand<T>,
	{
		let cmd = Arc::new(cmd);
		let (msg, receiver) = Command::from_command(cmd.clone());
		self
			.sender
			.send_async(msg)
			.await
			.into_report()
			.change_context_lazy(|| cmd.create_error())?;

		match receiver.await {
			Ok(r) => r,
			Err(e) => Err(report!(e).change_context(cmd.create_error())),
		}
	}

	pub async fn entity(
		&self,
		domain: impl Into<Arc<str>>,
		entity_id: impl Into<Arc<str>>,
	) -> error_stack::Result<EntityTopic, EntityCommandError> {
		let result = self
			.command(EntityCommand::new(domain.into(), entity_id.into()))
			.await?;

		Ok(EntityTopic::new(self.clone(), result.topics))
	}

	pub(crate) async fn publish(
		&self,
		topic: impl Into<Arc<str>>,
		payload: impl Into<Arc<[u8]>>,
		retained: bool,
		qos: MqttQosLevel,
	) -> error_stack::Result<(), PublishCommandError> {
		self
			.command(PublishCommand::new(
				topic.into(),
				payload.into(),
				retained,
				qos,
			))
			.await?;

		Ok(())
	}

	pub(crate) async fn subscribe(
		&self,
		topic: impl Into<Arc<str>>,
		qos: MqttQosLevel,
	) -> error_stack::Result<Subscription, SubscribeCommandError> {
		let topic = topic.into();
		let result = self
			.command(SubscribeCommand::new(topic.clone(), qos))
			.await?;

		Ok(Subscription {
			topic,
			qos,
			token: result.token,
			stream: result.receiver.into_stream(),
		})
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
