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
use opentelemetry::trace::{SpanContext, TraceContextExt};
use std::{sync::Arc, thread, time::Duration};
use thiserror::Error;
use tokio::{select, task::LocalSet};
use tracing::{field, instrument, span, Instrument, Level, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

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
	pub(super) span_context: SpanContext,
}

impl<T: MqttClient> InnerClient<T> {
	fn new(client: T, topics: TopicsConfig, span_context: SpanContext) -> Self {
		InnerClient {
			client,
			topics,
			router: Router::new(),
			subscriptions: Subscriptions::new(),
			span_context,
		}
	}

	async fn run(mut self, receiver: flume::Receiver<Command>) {
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
		// TODO: Trace?
		if let Some((_, Some(key))) = self.router.remove(tok) {
			// TODO: Log error
			let _ = self.client.unsubscribe(key).await;
		}
	}

	async fn handle_command(&mut self, cmd: Command) {
		// TODO: Trace?
		cmd.run(self).await
	}

	async fn handle_message(&mut self, msg: MqttReceivedMessage<T>) {
		// TODO: Trace?
		// let client_span_id = Span::current().id();

		let topic = msg.topic();
		let matches = self.router.matches(topic);
		if matches.len() == 0 {
			return;
		}

		let message_span = msg.span().clone();
		message_span.add_link(self.span_context.clone());

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
	let span = Span::current();
	let spawn_span_cx = span.context().span().span_context().clone();
	let (result_sender, result_receiver) = tokio::sync::oneshot::channel();

	thread::Builder::new()
		.name(format!("mqtt-{}-hass", options.application_name.slug()))
		.spawn({
			let parent = span;

			move || {
				let span = {
					let span = span!(
						parent: parent,
						Level::DEBUG,
						"InnerClient::thread_start",
						provider.name = %P::NAME,
						client.id = field::Empty,
					);
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

				let Ok(client) = local.block_on(&rt, {
				let span = span.exit();
				let span_clone = span.clone();
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
							return Err(());
						}
					};

					span_clone.record("client.id", &client_id);
					let client = InnerClient::new(mqtt_client, topics, spawn_span_cx);

					let _ = result_sender.send(Ok((sender, client_id.into())));
					Ok(client)
				}
				.instrument(span)
			}) else {
				return;
			};

				// run forever
				local.block_on(&rt, client.run(receiver));

				// ensure it lives til this point
				drop((rt_guard, local_guard));
			}
		})
		.map_err(ConnectError::spawn_thread)?;

	match result_receiver.await {
		Ok(Ok(sender)) => Ok(sender),
		Ok(Err(e)) => Err(e),
		Err(e) => Err(ConnectError::connect(e)),
	}
}
