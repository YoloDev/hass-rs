use crate::{
	client::{command::Command, subscription::Subscriptions, Message},
	mqtt::{HassMqttConnection, MqttProviderExt},
	router::Router,
	topics::TopicsConfig,
	HassMqttOptions,
};
use futures::{pin_mut, StreamExt};
use hass_dyn_error::DynError;
use hass_mqtt_provider::{
	MqttClient, MqttDisconnectBuilder, MqttMessage, MqttProvider, MqttReceivedMessage,
};
use std::{sync::Arc, thread, time::Duration};
use thiserror::Error;
use tokio::{select, task::LocalSet};
use tracing::{field, instrument, span, Level, Span};

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

pub(crate) struct InnerClient<T: MqttClient> {
	pub(super) client: T,
	pub(super) topics: TopicsConfig,
	pub(super) router: Router<T::SubscriptionKey, flume::Sender<Message>>,
	pub(super) subscriptions: Subscriptions,
}

impl<T: MqttClient> InnerClient<T> {
	fn new(client: T, topics: TopicsConfig) -> Self {
		InnerClient {
			client,
			topics,
			router: Router::new(),
			subscriptions: Subscriptions::new(),
		}
	}

	#[instrument(
		level = Level::DEBUG,
		name = "InnerClient::run",
		skip_all,
		fields(
			provider.name = %<T::Provider>::NAME,
			client.id = %self.client.client_id(),
		)
	)]
	async fn run(mut self, receiver: flume::Receiver<Command>) {
		// TODO: don't use the events helper, use select instead
		let receiver = receiver.into_stream().fuse();
		let messages = self.client.messages().fuse();

		pin_mut!(receiver);
		pin_mut!(messages);

		loop {
			select! {
				tok = self.subscriptions.dropped() => self.handle_unsubscribe(tok).await,
				Some(cmd) = receiver.next() => self.handle_command(cmd).await,
				Some(msg) = messages.next() => self.handle_message(msg).await,
				else => break,
			}
		}

		let _ = self
			.client
			.disconnect()
			.after(Duration::from_secs(10))
			.publish_last_will(true)
			.await;
	}

	async fn handle_unsubscribe(&mut self, tok: RouteId) {
		if let Some((_, Some(key))) = self.router.remove(tok) {
			// TODO: Log error
			let _ = self.client.unsubscribe(key).await;
		}
	}

	async fn handle_command(&mut self, cmd: Command) {
		cmd.run(self).await
	}

	async fn handle_message(&mut self, msg: MqttReceivedMessage<T>) {
		let client_span = Span::current();

		let topic = msg.topic();
		let matches = self.router.matches(topic);
		if matches.len() == 0 {
			return;
		}

		let message_span = msg.span().clone();
		message_span.follows_from(client_span);

		let message = Message {
			topic: topic.into(),
			payload: msg.payload().into(),
			retained: msg.retained(),
			span: message_span,
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
}

#[instrument(
	level = Level::DEBUG,
	name = "InnerClient::spawn"
	skip_all,
	fields(
		provider.name = %P::NAME,
	)
)]
pub(super) async fn spawn<P: MqttProvider>(
	options: HassMqttOptions,
) -> Result<(flume::Sender<Command>, Arc<str>), ConnectError> {
	let spawn_span = Span::current().id();
	let (result_sender, result_receiver) = tokio::sync::oneshot::channel();

	thread::Builder::new()
		.name(format!("mqtt-{}-hass", options.application_name.slug()))
		.spawn(move || {
			let span = {
				let span = span!(
					parent: None,
					Level::DEBUG,
					"InnerClient::thread",
					provider.name = %P::NAME,
					client.id = field::Empty,
				);
				span.follows_from(spawn_span);
				span.entered()
			};

			let (sender, receiver) = flume::unbounded();
			let local = LocalSet::new();

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

			let rt_guard = rt.enter();
			let local_guard = local.enter();
			local.block_on(&rt, {
				let span = &span;
				async move {
					let HassMqttConnection {
						topics,
						client: mqtt_client,
						client_id,
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

					span.record("client.id", &client_id);
					let client = InnerClient::new(mqtt_client, topics);

					let _ = result_sender.send(Ok((sender, client_id.into())));
					client.run(receiver).await;
				}
			});

			// ensure it lives til this point
			drop((rt_guard, local_guard, span));
		})
		.map_err(ConnectError::spawn_thread)?;

	match result_receiver.await {
		Ok(Ok(sender)) => Ok(sender),
		Ok(Err(e)) => Err(e),
		Err(e) => Err(ConnectError::connect(e)),
	}
}
