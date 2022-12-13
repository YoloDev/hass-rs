mod entity;
mod publish;
mod subscribe;

use super::{inner::InnerClient, QosLevel};
use async_trait::async_trait;
use hass_mqtt_provider::MqttClient;
use std::sync::Arc;
use tokio::sync::oneshot;

pub(super) use entity::EntityCommand;
pub(super) use publish::PublishCommand;
pub(super) use subscribe::SubscribeCommand;

#[async_trait(?Send)]
pub(super) trait ClientCommand {
	type Result: Send + Sync + 'static;
	type Error: std::error::Error + Send + Sync + 'static;

	async fn run<T: MqttClient>(
		&self,
		client: &mut InnerClient,
		mqtt: &T,
	) -> Result<Self::Result, Self::Error>;

	fn create_error(&self, source: impl std::error::Error + Send + Sync + 'static) -> Self::Error;
}

pub(super) type CommandResult<T> =
	Result<<T as ClientCommand>::Result, <T as ClientCommand>::Error>;
pub(super) type CommandResultSender<T> = oneshot::Sender<CommandResult<T>>;
pub(super) type CommandResultReceiver<T> = oneshot::Receiver<CommandResult<T>>;

pub(super) trait FromClientCommand<T: ClientCommand>: Sized {
	fn from_command(command: Arc<T>) -> (Self, CommandResultReceiver<T>);
}

macro_rules! commands {
	($vis:vis enum $name:ident {
		$($variant:ident),*$(,)?
	}) => {
		#[allow(clippy::enum_variant_names)]
		$vis enum $name {
			$($variant(Arc<$variant>, CommandResultSender<$variant>),)*
		}

		$(
			impl FromClientCommand<$variant> for $name {
				fn from_command(command: Arc<$variant>) -> (Self, CommandResultReceiver<$variant>) {
					let (tx, rx) = oneshot::channel();

					(Self::$variant(command, tx), rx)
				}
			}
		)*

		impl $name {
			pub(super) fn from_command<T>(command: Arc<T>) -> (Self, CommandResultReceiver<T>)
			where
				T: ClientCommand,
				Self: FromClientCommand<T>,
			{
				<Self as FromClientCommand<T>>::from_command(command)
			}

			pub(super) async fn run<T: MqttClient>(
				self,
				client: &mut InnerClient,
				mqtt: &T,
			) {
				match self {
					$(
						Self::$variant(command, tx) => {
							// TODO: tracing
							let result = command.run(client, mqtt).await;

							if let Err(_err) = tx.send(result) {
								// TODO: log
							}
						}
					)*
				}
			}
		}
	};
}

commands! {
	pub(super) enum Command {
		EntityCommand,
		PublishCommand,
		SubscribeCommand,
	}
}

pub(super) fn entity(domain: Arc<str>, entity_id: Arc<str>) -> EntityCommand {
	EntityCommand::new(domain, entity_id)
}

pub(super) fn publish(
	topic: Arc<str>,
	payload: Arc<[u8]>,
	retained: bool,
	qos: QosLevel,
) -> PublishCommand {
	PublishCommand::new(topic, payload, retained, qos)
}

pub(super) fn subscribe(topic: Arc<str>, qos: QosLevel) -> SubscribeCommand {
	SubscribeCommand::new(topic, qos)
}
