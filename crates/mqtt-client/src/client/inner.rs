use crate::{
	client::{command::Command, subscription::Subscriptions, Message},
	error::DynError,
	mqtt::{HassMqttConnection, MqttProviderExt},
	router::Router,
	topics::TopicsConfig,
	HassMqttOptions,
};
use futures::{pin_mut, StreamExt};
use hass_mqtt_provider::{MqttClient, MqttMessage, MqttProvider};
use std::{thread, time::Duration};
use thiserror::Error;
use tokio::select;

type RouteId = generational_arena::Index;

#[derive(Debug, Error)]
pub enum ConnectError {
	#[error("failed to connect to MQTT broker")]
	Connect {
		#[cfg_attr(provide_any, backtrace)]
		source: DynError,
	},

	#[error("falied to spawn MQTT thread")]
	SpawnThread {
		#[cfg_attr(provide_any, backtrace)]
		source: DynError,
	},

	#[error("failed to create async MQTT runtime")]
	CreateRuntime {
		#[cfg_attr(provide_any, backtrace)]
		source: DynError,
	},
}

impl ConnectError {
	fn connect(source: impl std::error::Error + Send + Sync + 'static) -> Self {
		Self::Connect {
			source: DynError::new(source),
		}
	}

	fn spawn_thread(source: impl std::error::Error + Send + Sync + 'static) -> Self {
		Self::SpawnThread {
			source: DynError::new(source),
		}
	}

	fn create_runtime(source: impl std::error::Error + Send + Sync + 'static) -> Self {
		Self::CreateRuntime {
			source: DynError::new(source),
		}
	}
}

pub(super) struct InnerClient {
	pub(super) topics: TopicsConfig,
	pub(super) router: Router<flume::Sender<Message>>,
	pub(super) subscriptions: Subscriptions,
}

impl InnerClient {
	fn new(topics: TopicsConfig) -> Self {
		InnerClient {
			topics,
			router: Router::new(),
			subscriptions: Subscriptions::new(),
		}
	}

	async fn run<T: MqttClient>(mut self, client: T, receiver: flume::Receiver<Command>) {
		// TODO: don't use the events helper, use select instead
		let receiver = receiver.into_stream().fuse();
		let messages = client.messages().fuse();

		pin_mut!(receiver);
		pin_mut!(messages);

		loop {
			select! {
				tok = self.subscriptions.dropped() => self.handle_unsubscribe(tok, &client).await,
				Some(cmd) = receiver.next() => self.handle_command(cmd, &client).await,
				Some(msg) = messages.next() => self.handle_message(msg, &client).await,
				else => break,
			}
		}

		let _ = client.disconnect(Duration::from_secs(10), true).await;
	}

	async fn handle_unsubscribe<T: MqttClient>(&mut self, tok: RouteId, client: &T) {
		if let Some((_, Some(topic))) = self.router.remove(tok) {
			// TODO: Log error
			let _ = client.unsubscribe(&*topic).await;
		}
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

	pub(super) async fn spawn<P: MqttProvider>(
		options: HassMqttOptions,
	) -> Result<flume::Sender<Command>, ConnectError> {
		let (result_sender, result_receiver) = tokio::sync::oneshot::channel();

		thread::Builder::new()
			.name(format!("mqtt-{}-hass", options.application_name.slug()))
			.spawn(move || {
				let (sender, receiver) = flume::unbounded();
				let rt = match tokio::runtime::Builder::new_current_thread()
					.build()
					.map_err(ConnectError::create_runtime)
				{
					Ok(rt) => rt,
					Err(e) => {
						let _ = result_sender.send(Err(e));
						return;
					}
				};

				let guard = rt.enter();
				rt.block_on(async move {
					let HassMqttConnection {
						topics,
						client: mqtt_client,
					} = match <P as MqttProviderExt>::create_client(&options)
						.await
						.map_err(ConnectError::connect)
					{
						Ok(c) => c,
						Err(e) => {
							let _ = result_sender.send(Err(e));
							return;
						}
					};

					let client = InnerClient::new(topics);

					let _ = result_sender.send(Ok(sender));
					client.run(mqtt_client, receiver).await;
				});

				// ensure it lives til this point
				drop(guard);
			})
			.map_err(ConnectError::spawn_thread)?;

		match result_receiver.await {
			Ok(Ok(sender)) => Ok(sender),
			Ok(Err(e)) => Err(e),
			Err(e) => Err(ConnectError::connect(e)),
		}
	}
}
