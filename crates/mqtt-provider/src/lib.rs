use async_trait::async_trait;
use futures::stream::Stream;
use std::{
	fmt::{self, Write},
	future::IntoFuture,
	path::PathBuf,
	sync::Arc,
	time::Duration,
};
use tracing::{
	Span,
	span::{Entered, EnteredSpan},
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

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum MqttRetainHandling {
	/// Send retained messages at the time of the subscribe
	SendRetainedOnSubscribe = 0,
	/// Send retained messages on subscribe only if subscription is new
	SendRetainedOnNew = 1,
	/// Do not send retained messages at all
	DontSendRetained = 2,
}

/// Hints to the MQTT provider what version of the protocol to use.
/// The provider may choose to ignore this hint.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum MqttVersion {
	/// Default version (let the provider pick)
	Default = 0,
	/// Choose a version 3.x client
	V3 = 1,
	/// Choose a version 5.x client
	V5 = 2,
}

impl fmt::Display for MqttRetainHandling {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			MqttRetainHandling::SendRetainedOnSubscribe => f.write_char('0'),
			MqttRetainHandling::SendRetainedOnNew => f.write_char('1'),
			MqttRetainHandling::DontSendRetained => f.write_char('2'),
		}
	}
}

impl From<MqttRetainHandling> for u8 {
	fn from(qos: MqttRetainHandling) -> Self {
		qos as u8
	}
}

impl From<MqttRetainHandling> for i32 {
	fn from(qos: MqttRetainHandling) -> Self {
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
	const NAME: &'static str;

	type Client: MqttClient<Message = Self::Message>;
	type Message: MqttBuildableMessage<Client = Self::Client>;
	type Error: MqttProviderCreateError + std::error::Error + Send + Sync + 'static;

	#[allow(clippy::too_many_arguments)]
	async fn create(
		options: &impl AsMqttOptions,
		client_id: &str,
		online_message: Self::Message,
		offline_message: Self::Message,
	) -> Result<Self::Client, Self::Error>;
}

pub trait MqttClient: Sized {
	type Provider: MqttProvider<Client = Self>;
	type Message: MqttBuildableMessage<Client = Self>;
	type Messages: Stream<Item = MqttReceivedMessage<Self>>;
	type SubscriptionKey: Send + Sync + 'static;
	type PublishBuilder<'a>: MqttPublishBuilder + 'a
	where
		Self: 'a;
	type SubscribeBuilder<'a>: MqttSubscribeBuilder<SubscriptionKey = Self::SubscriptionKey> + 'a
	where
		Self: 'a;
	type UnsubscribeBuilder<'a>: MqttUnsubscribeBuilder + 'a
	where
		Self: 'a;
	type DisconnectBuilder<'a>: MqttDisconnectBuilder + 'a
	where
		Self: 'a;

	fn client_id(&self) -> Arc<str>;

	fn messages(&self) -> Self::Messages;

	fn publish(&self, message: Self::Message) -> Self::PublishBuilder<'_>;

	fn subscribe(&self, topic: impl Into<Arc<str>>, qos: QosLevel) -> Self::SubscribeBuilder<'_>;

	fn unsubscribe(&self, key: Self::SubscriptionKey) -> Self::UnsubscribeBuilder<'_>;

	fn disconnect(&self) -> Self::DisconnectBuilder<'_>;
}

pub trait MqttPublishBuilder: IntoFuture<Output = Result<(), Self::Error>> {
	type Error: std::error::Error + Send + Sync + 'static;
}

pub trait MqttSubscribeBuilder:
	IntoFuture<Output = Result<Self::SubscriptionKey, Self::Error>>
{
	type SubscriptionKey: Send + Sync + 'static;
	type Error: std::error::Error + Send + Sync + 'static;

	fn no_local(self, on: bool) -> Self;
	fn retain_handling(self, handling: MqttRetainHandling) -> Self;
}

pub trait MqttUnsubscribeBuilder: IntoFuture<Output = Result<(), Self::Error>> {
	type Error: std::error::Error + Send + Sync + 'static;
}

pub trait MqttDisconnectBuilder: IntoFuture<Output = Result<(), Self::Error>> {
	type Error: std::error::Error + Send + Sync + 'static;

	fn publish_last_will(self, on: bool) -> Self;
	fn after(self, timeout: Duration) -> Self;
}

pub trait MqttMessage {
	type Client: MqttClient;

	fn topic(&self) -> &str;
	fn payload(&self) -> &[u8];
	fn retained(&self) -> bool;
	fn qos(&self) -> QosLevel;
}

pub trait MqttBuildableMessage: MqttMessage {
	type Builder: MqttMessageBuilder<Message = Self>;

	fn builder() -> Self::Builder;
}

pub struct MqttReceivedMessage<T: MqttClient> {
	message: T::Message,
	span: Span,
}

impl<T: MqttClient> MqttMessage for MqttReceivedMessage<T> {
	type Client = T;

	#[inline]
	fn topic(&self) -> &str {
		MqttMessage::topic(&self.message)
	}

	#[inline]
	fn payload(&self) -> &[u8] {
		MqttMessage::payload(&self.message)
	}

	#[inline]
	fn retained(&self) -> bool {
		MqttMessage::retained(&self.message)
	}

	#[inline]
	fn qos(&self) -> QosLevel {
		MqttMessage::qos(&self.message)
	}
}

impl<T: MqttClient> MqttReceivedMessage<T> {
	pub fn new(message: T::Message, span: Span) -> Self {
		Self { message, span }
	}

	pub fn span(&self) -> &Span {
		&self.span
	}

	pub fn into_parts(self) -> (T::Message, Span) {
		(self.message, self.span)
	}

	pub fn enter(&self) -> Entered<'_> {
		self.span.enter()
	}

	pub fn entered(self) -> EnteredMessage<T>
	where
		Self: Sized,
	{
		let (message, span) = self.into_parts();
		let span = span.entered();

		EnteredMessage {
			message,
			_span: span,
		}
	}
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
	#[cfg(feature = "ssl")]
	#[cfg_attr(doc_cfg, doc(cfg(feature = "ssl")))]
	pub tls: bool,
	pub auth: Option<MqttAuthOptions>,
	pub persitence: PathBuf,
	pub version: MqttVersion,
}

impl MqttOptions {
	pub fn new(host: impl Into<String>, persitence: PathBuf) -> Self {
		MqttOptions {
			host: host.into(),
			port: 1883,
			#[cfg(feature = "ssl")]
			tls: false,
			auth: None,
			persitence,
			version: MqttVersion::Default,
		}
	}

	#[cfg(feature = "ssl")]
	#[cfg_attr(doc_cfg, doc(cfg(feature = "tls")))]
	pub fn new_tls(host: impl Into<String>, persitence: PathBuf) -> Self {
		MqttOptions {
			host: host.into(),
			port: 8883,
			tls: true,
			auth: None,
			persitence,
			version: MqttVersion::Default,
		}
	}

	pub fn port(&mut self, port: u16) -> &mut Self {
		self.port = port;
		self
	}

	#[cfg(feature = "ssl")]
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

	pub fn version(&mut self, version: MqttVersion) -> &mut Self {
		self.version = version;
		self
	}
}

#[derive(Clone)]
pub struct MqttAuthOptions {
	pub username: String,
	pub password: String,
}

pub struct EnteredMessage<T: MqttClient> {
	message: T::Message,
	_span: EnteredSpan,
}

impl<T: MqttClient> MqttMessage for EnteredMessage<T> {
	type Client = T;

	#[inline]
	fn topic(&self) -> &str {
		MqttMessage::topic(&self.message)
	}

	#[inline]
	fn payload(&self) -> &[u8] {
		MqttMessage::payload(&self.message)
	}

	#[inline]
	fn retained(&self) -> bool {
		MqttMessage::retained(&self.message)
	}

	#[inline]
	fn qos(&self) -> QosLevel {
		MqttMessage::qos(&self.message)
	}
}
