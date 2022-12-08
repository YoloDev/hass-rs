use crate::{
	topics::{ApplicationName, NodeId, TopicsConfig},
	MqttQosLevel,
};
use async_trait::async_trait;
use error_stack::ResultExt;
use futures::stream::Stream;

#[cfg(feature = "paho")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "paho")))]
mod paho;

#[cfg(feature = "paho")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "paho")))]
pub use paho::PahoMqtt;

mod sealed {
	pub trait Sealed {}
}

pub trait MqttProviderCreateError: std::error::Error + Send + Sync + 'static {
	fn create_message(kind: impl Into<String>) -> Self;
}

#[async_trait(?Send)]
pub trait MqttProvider: sealed::Sealed {
	type Client: MqttClient<Message = Self::Message>;
	type Message: MqttMessage;
	type Error: MqttProviderCreateError;

	#[allow(clippy::too_many_arguments)]
	async fn create(
		options: &crate::options::MqttOptions,
		client_id: &str,
		application_name: &ApplicationName,
		node_id: &NodeId,
		topics: &TopicsConfig,
		online_message: Self::Message,
		offline_message: Self::Message,
	) -> error_stack::Result<Self::Client, Self::Error>;
}

pub(crate) struct HassMqttConnection<T>
where
	T: MqttClient,
{
	pub(crate) topics: TopicsConfig,
	pub(crate) client: T,
}

#[async_trait(?Send)]
pub(crate) trait MqttProviderExt: MqttProvider {
	async fn create_client(
		options: &crate::HassMqttOptions,
	) -> error_stack::Result<HassMqttConnection<Self::Client>, Self::Error> {
		let node_id = NodeId::new(&*options.node_id);
		let client_id = format!("{}_{}", options.application_name.slug(), options.node_id);
		let topics = TopicsConfig::new(
			options
				.private_prefix
				.as_deref()
				.unwrap_or_else(|| options.application_name.slug()),
			&*options.discovery_prefix,
			node_id.clone(),
		);
		let online_message = topics
			.online_message()
			.change_context(Self::Error::create_message("online"))?;
		let offline_message = topics
			.offline_message()
			.change_context(Self::Error::create_message("offline"))?;

		let client = Self::create(
			&options.mqtt,
			&client_id,
			&options.application_name,
			&node_id,
			&topics,
			online_message,
			offline_message,
		)
		.await?;
		Ok(HassMqttConnection { topics, client })
	}
}

#[async_trait(?Send)]
impl<T: MqttProvider> MqttProviderExt for T {}

#[async_trait(?Send)]
pub trait MqttClient: sealed::Sealed {
	type Message: MqttMessage;
	type Messages: Stream<Item = Self::Message>;
	type PublishError: std::error::Error + Send + Sync + 'static;
	type SubscribeError: std::error::Error + Send + Sync + 'static;
	type UnsubscribeError: std::error::Error + Send + Sync + 'static;
	type DisconnectError: std::error::Error + Send + Sync + 'static;

	fn messages(&self) -> Self::Messages;

	async fn publish(&self, message: Self::Message) -> error_stack::Result<(), Self::PublishError>;

	async fn subscribe(
		&self,
		topic: impl Into<String>,
		qos: MqttQosLevel,
	) -> error_stack::Result<(), Self::SubscribeError>;

	async fn unsubscribe(
		&self,
		topic: impl Into<String>,
	) -> error_stack::Result<(), Self::UnsubscribeError>;

	async fn disconnect(
		&self,
		timeout: std::time::Duration,
		publish_last_will: bool,
	) -> error_stack::Result<(), Self::DisconnectError>;
}

pub trait MqttMessage: sealed::Sealed + Clone {
	type Builder: MqttMessageBuilder<Message = Self>;

	fn builder() -> Self::Builder;
	fn topic(&self) -> &str;
	fn payload(&self) -> &[u8];
	fn retained(&self) -> bool;
}
pub trait MqttMessageBuilder: sealed::Sealed {
	type Message: MqttMessage;
	type Error: std::error::Error;

	fn topic(self, topic: impl Into<String>) -> Self;
	fn payload(self, payload: impl Into<Vec<u8>>) -> Self;
	fn qos(self, qos: MqttQosLevel) -> Self;
	fn retain(self, retain: bool) -> Self;
	fn build(self) -> error_stack::Result<Self::Message, Self::Error>;
}
