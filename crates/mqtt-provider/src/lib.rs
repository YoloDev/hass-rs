use async_trait::async_trait;
use futures::stream::Stream;
use std::{
	fmt::{self, Write},
	path::PathBuf,
};

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum QosLevel {
	AtLeastOnce = 0,
	AtMostOnce = 1,
	ExactlyOnce = 2,
}

impl fmt::Display for QosLevel {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			QosLevel::AtLeastOnce => f.write_char('0'),
			QosLevel::AtMostOnce => f.write_char('1'),
			QosLevel::ExactlyOnce => f.write_char('2'),
		}
	}
}

impl From<QosLevel> for u8 {
	fn from(qos: QosLevel) -> Self {
		qos as u8
	}
}

impl From<QosLevel> for i32 {
	fn from(qos: QosLevel) -> Self {
		qos as i32
	}
}

pub trait MqttProviderCreateError {
	fn create_message(
		kind: impl Into<String>,
		source: impl std::error::Error + Send + Sync + 'static,
	) -> Self;
}

#[async_trait(?Send)]
pub trait MqttProvider {
	type Client: MqttClient<Message = Self::Message>;
	type Message: MqttMessage;
	type Error: MqttProviderCreateError + std::error::Error + Send + Sync + 'static;

	#[allow(clippy::too_many_arguments)]
	async fn create(
		options: &impl AsMqttOptions,
		client_id: &str,
		online_message: Self::Message,
		offline_message: Self::Message,
	) -> Result<Self::Client, Self::Error>;
}

#[async_trait(?Send)]
pub trait MqttClient {
	type Message: MqttMessage;
	type Messages: Stream<Item = Self::Message>;
	type PublishError: std::error::Error + Send + Sync + 'static;
	type SubscribeError: std::error::Error + Send + Sync + 'static;
	type UnsubscribeError: std::error::Error + Send + Sync + 'static;
	type DisconnectError: std::error::Error + Send + Sync + 'static;

	fn messages(&self) -> Self::Messages;

	async fn publish(&self, message: Self::Message) -> Result<(), Self::PublishError>;

	async fn subscribe(
		&self,
		topic: impl Into<String>,
		qos: QosLevel,
	) -> Result<(), Self::SubscribeError>;

	async fn unsubscribe(&self, topic: impl Into<String>) -> Result<(), Self::UnsubscribeError>;

	async fn disconnect(
		&self,
		timeout: std::time::Duration,
		publish_last_will: bool,
	) -> Result<(), Self::DisconnectError>;
}

pub trait MqttMessage: Clone {
	type Builder: MqttMessageBuilder<Message = Self>;

	fn builder() -> Self::Builder;
	fn topic(&self) -> &str;
	fn payload(&self) -> &[u8];
	fn retained(&self) -> bool;
}
pub trait MqttMessageBuilder {
	type Message: MqttMessage;
	type Error: std::error::Error + Send + Sync + 'static;

	fn topic(self, topic: impl Into<String>) -> Self;
	fn payload(self, payload: impl Into<Vec<u8>>) -> Self;
	fn qos(self, qos: QosLevel) -> Self;
	fn retain(self, retain: bool) -> Self;
	fn build(self) -> Result<Self::Message, Self::Error>;
}

pub trait AsMqttOptions {
	type Error: std::error::Error + Send + Sync + 'static;

	fn mqtt_options(&self) -> Result<MqttOptions, Self::Error>;
}

impl<T, E> AsMqttOptions for T
where
	T: TryInto<MqttOptions, Error = E> + Clone,
	E: std::error::Error + Send + Sync + 'static,
{
	type Error = E;

	fn mqtt_options(&self) -> Result<MqttOptions, E> {
		self.clone().try_into()
	}
}

#[derive(Clone)]
pub struct MqttOptions {
	pub host: String,
	pub port: u16,
	#[cfg(feature = "tls")]
	#[cfg_attr(doc_cfg, doc(cfg(feature = "tls")))]
	pub tls: bool,
	pub auth: Option<MqttAuthOptions>,
	pub persitence: PathBuf,
}

impl MqttOptions {
	pub fn new(host: impl Into<String>, persitence: PathBuf) -> Self {
		MqttOptions {
			host: host.into(),
			port: 1883,
			#[cfg(feature = "tls")]
			tls: false,
			auth: None,
			persitence,
		}
	}

	#[cfg(feature = "tls")]
	#[cfg_attr(doc_cfg, doc(cfg(feature = "tls")))]
	pub fn new_tls(host: impl Into<String>, persitence: PathBuf) -> Self {
		MqttOptions {
			host: host.into(),
			port: 8883,
			tls: true,
			auth: None,
			persitence,
		}
	}

	pub fn port(&mut self, port: u16) -> &mut Self {
		self.port = port;
		self
	}

	#[cfg(feature = "tls")]
	#[cfg_attr(doc_cfg, doc(cfg(feature = "tls")))]
	pub fn tls(&mut self, tls: bool) -> &mut Self {
		self.tls = tls;
		self
	}

	pub fn auth(&mut self, username: impl Into<String>, password: impl Into<String>) -> &mut Self {
		self.auth = Some(MqttAuthOptions {
			username: username.into(),
			password: password.into(),
		});
		self
	}
}

#[derive(Clone)]
pub struct MqttAuthOptions {
	pub username: String,
	pub password: String,
}
