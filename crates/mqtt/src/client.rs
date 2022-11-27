use crate::HassMqttOptions;
use tokio::sync::{mpsc, oneshot};

enum Command {}

pub enum ConnectError {}
impl From<oneshot::error::RecvError> for ConnectError {
	fn from(_: oneshot::error::RecvError) -> Self {
		todo!()
	}
}

struct Client {
	discovery_prefix: String,
	private_prefix: String,
	node_id: String,
}

impl Client {
	async fn run(
		options: HassMqttOptions,
		ready_channel: oneshot::Sender<Result<mpsc::UnboundedSender<Command>, ConnectError>>,
	) {
		// TODO: Connecto to mqtt...
		let mut client = Client {
			discovery_prefix: options.discovery_prefix,
			private_prefix: options.private_prefix,
			node_id: options.node_id,
		};

		let (sender, mut receiver) = mpsc::unbounded_channel();
		let _ = ready_channel.send(Ok(sender));

		while let Some(cmd) = receiver.recv().await {
			client.handle(cmd).await
		}
	}

	async fn spawn(options: HassMqttOptions) -> Result<mpsc::UnboundedSender<Command>, ConnectError> {
		let (ready_sender, ready_receiver) = oneshot::channel();
		tokio::spawn(Client::run(options, ready_sender));

		ready_receiver.await?
	}

	async fn handle(&mut self, cmd: Command) {}
}

#[derive(Clone)]
pub struct HassMqttClient {
	sender: mpsc::UnboundedSender<Command>,
}

impl HassMqttClient {
	pub async fn new(options: HassMqttOptions) -> Result<Self, ConnectError> {
		let sender = Client::spawn(options).await?;
		Ok(Self { sender })
	}
}

impl HassMqttOptions {
	pub async fn build(&self) -> Result<HassMqttClient, ConnectError> {
		HassMqttClient::new(self.clone()).await
	}
}
