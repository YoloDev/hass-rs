use crate::{
	options::MqttPersistence,
	topics::{ApplicationName, DiscoveryTopicConfig, NodeId, PrivateTopicConfig},
};

use super::{sealed, MqttClient, MqttMessage, MqttMessageBuilder, MqttProvider};
use async_trait::async_trait;
use dirs::{cache_dir, state_dir};
use error_stack::{IntoReport, ResultExt};
use futures::{stream::FusedStream, Stream};
use paho_mqtt::{AsyncClient, AsyncReceiver, Message};
use pin_project::pin_project;
use std::{
	convert::Infallible,
	path::{Path, PathBuf},
	pin::Pin,
	task::{Context, Poll},
	time::Duration,
};
use thiserror::Error;
use tokio::net::lookup_host;

pub struct PahoMqtt;

pub struct PahoClient {
	client: AsyncClient,
	messages: PahoStream,
}

#[pin_project]
#[derive(Clone)]
pub struct PahoStream {
	#[pin]
	inner: AsyncReceiver<Option<Message>>,
}

#[derive(Debug, Clone, Error)]
pub enum PahoProviderConnectError {
	#[error("failed to create MQTT client")]
	Client,

	#[error("failed to connect to MQTT broker")]
	Connect,

	#[error("faild to find state or cache directory")]
	StateDir,

	#[error("falied to resolve host: {host}:{port}")]
	ResolveHost { host: String, port: u16 },

	#[error("failed to create MQTT message: {kind}")]
	Message { kind: String },
}

impl PahoProviderConnectError {
	fn resolve_host(host: &str, port: u16) -> Self {
		PahoProviderConnectError::ResolveHost {
			host: host.into(),
			port,
		}
	}
}

impl super::MqttProviderCreateError for PahoProviderConnectError {
	fn create_message(kind: impl Into<String>) -> Self {
		Self::Message { kind: kind.into() }
	}
}

impl sealed::Sealed for PahoMqtt {}
#[async_trait(?Send)]
impl MqttProvider for PahoMqtt {
	type Client = PahoClient;
	type Message = Message;
	type Error = PahoProviderConnectError;

	async fn create(
		options: &crate::options::MqttOptions,
		client_id: &str,
		application_name: &ApplicationName,
		node_id: &NodeId,
		_discovery_topic: &DiscoveryTopicConfig,
		_private_topic: &PrivateTopicConfig,
		online_message: Self::Message,
		offline_message: Self::Message,
	) -> error_stack::Result<Self::Client, Self::Error> {
		create_client(
			options,
			client_id,
			application_name,
			node_id,
			online_message,
			offline_message,
		)
		.await
	}
}

impl sealed::Sealed for PahoClient {}

#[async_trait(?Send)]
impl MqttClient for PahoClient {
	type Message = Message;
	type Messages = PahoStream;
	type DisconnectError = paho_mqtt::Error;

	async fn disconnect(
		&self,
		timeout: std::time::Duration,
		publish_last_will: bool,
	) -> error_stack::Result<(), Self::DisconnectError> {
		let mut builder = paho_mqtt::DisconnectOptionsBuilder::new();
		builder.timeout(timeout);
		if publish_last_will {
			builder.publish_will_message();
		}

		AsyncClient::disconnect(&self.client, builder.finalize())
			.await
			.into_report()
			.map(|_| ())
	}

	fn messages(&self) -> Self::Messages {
		self.messages.clone()
	}
}

impl sealed::Sealed for paho_mqtt::Message {}
impl MqttMessage for paho_mqtt::Message {
	type Builder = paho_mqtt::MessageBuilder;

	fn builder() -> Self::Builder {
		paho_mqtt::MessageBuilder::new()
	}
}

impl sealed::Sealed for paho_mqtt::MessageBuilder {}
impl MqttMessageBuilder for paho_mqtt::MessageBuilder {
	type Message = paho_mqtt::Message;
	type Error = Infallible;

	fn topic(self, topic: impl Into<String>) -> Self {
		paho_mqtt::MessageBuilder::topic(self, topic)
	}

	fn payload(self, payload: impl Into<Vec<u8>>) -> Self {
		paho_mqtt::MessageBuilder::payload(self, payload)
	}

	fn qos(self, qos: crate::MqttQosLevel) -> Self {
		paho_mqtt::MessageBuilder::qos(
			self,
			match qos {
				crate::MqttQosLevel::AtMostOnce => paho_mqtt::QOS_0,
				crate::MqttQosLevel::AtLeastOnce => paho_mqtt::QOS_1,
				crate::MqttQosLevel::ExactlyOnce => paho_mqtt::QOS_2,
			},
		)
	}

	fn retain(self, retain: bool) -> Self {
		paho_mqtt::MessageBuilder::retained(self, retain)
	}

	fn build(self) -> error_stack::Result<Self::Message, Self::Error> {
		Ok(self.finalize())
	}
}

impl Stream for PahoStream {
	type Item = Message;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		loop {
			match self.as_mut().project().inner.poll_next(cx) {
				Poll::Ready(Some(Some(message))) => return Poll::Ready(Some(message)),
				Poll::Ready(Some(None)) => continue,
				Poll::Ready(None) => return Poll::Ready(None),
				Poll::Pending => return Poll::Pending,
			}
		}
	}
}

impl FusedStream for PahoStream {
	fn is_terminated(&self) -> bool {
		FusedStream::is_terminated(&self.inner)
	}
}

fn join_persistence_file(
	dir: &Path,
	application_name: &ApplicationName,
	node_id: &NodeId,
) -> PathBuf {
	dir.join(format!("{}.{}.mqtt", application_name.slug(), node_id))
}

fn as_create_options(
	options: &crate::options::MqttOptions,
	client_id: &str,
	application_name: &ApplicationName,
	node_id: &NodeId,
) -> error_stack::Result<paho_mqtt::CreateOptions, PahoProviderConnectError> {
	let builder = paho_mqtt::CreateOptionsBuilder::new()
		.client_id(client_id)
		.send_while_disconnected(true);

	let persistence_file = match &options.persitence {
		MqttPersistence::Default => state_dir()
			.or_else(cache_dir)
			.map(|dir| join_persistence_file(&dir, application_name, node_id))
			.ok_or(PahoProviderConnectError::StateDir)?,
		MqttPersistence::File(d) => d.clone(),
		MqttPersistence::Directory(d) => join_persistence_file(d, application_name, node_id),
	};

	let builder = builder.persistence(persistence_file);

	Ok(builder.finalize())
}

pub(crate) async fn create_client(
	options: &crate::options::MqttOptions,
	client_id: &str,
	application_name: &ApplicationName,
	node_id: &NodeId,
	online_message: paho_mqtt::Message,
	offline_message: paho_mqtt::Message,
) -> error_stack::Result<PahoClient, PahoProviderConnectError> {
	let mut client = paho_mqtt::AsyncClient::new(as_create_options(
		options,
		client_id,
		application_name,
		node_id,
	)?)
	.into_report()
	.change_context(PahoProviderConnectError::Client)?;

	let mut builder = paho_mqtt::ConnectOptionsBuilder::new();
	let hosts = lookup_host((&*options.host, options.port))
		.await
		.into_report()
		.change_context(PahoProviderConnectError::resolve_host(
			&options.host,
			options.port,
		))?
		.map(|addr| format!("tcp://{addr}"))
		.collect::<Vec<_>>();

	builder
		.server_uris(&hosts)
		.automatic_reconnect(Duration::from_secs(5), Duration::from_secs(60 * 5));

	if options.tls {
		builder.ssl_options(paho_mqtt::SslOptions::new());
	}

	if let Some(auth) = &options.auth {
		builder.user_name(auth.username.clone());
		builder.password(auth.password.clone());
	}

	builder.will_message(offline_message);
	client.set_connected_callback(move |c| {
		// TODO: log
		let _ = c.publish(online_message.clone()).wait();
	});

	let messages = PahoStream {
		inner: client.get_stream(100),
	};

	client
		.connect(builder.finalize())
		.await
		.into_report()
		.change_context(PahoProviderConnectError::Connect)?;

	Ok(PahoClient { client, messages })
}
