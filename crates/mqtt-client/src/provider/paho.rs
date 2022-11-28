use crate::topics::{DiscoveryTopicConfig, PrivateTopicConfig};

use super::{sealed, MqttClient, MqttMessage, MqttMessageBuilder, MqttProvider};
use async_trait::async_trait;
use error_stack::IntoReport;
use paho_mqtt::AsyncClient;
use std::convert::Infallible;
use thiserror::Error;

pub struct PahoMqtt;

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
	type Client = AsyncClient;
	type Message = paho_mqtt::Message;
	type Error = PahoProviderConnectError;

	async fn create(
		options: &crate::options::MqttOptions,
		discovery_topic: &DiscoveryTopicConfig,
		private_topic: &PrivateTopicConfig,
		online_message: Self::Message,
		offline_message: Self::Message,
	) -> error_stack::Result<Self::Client, Self::Error> {
		todo!()
	}
}

impl sealed::Sealed for AsyncClient {}

#[async_trait(?Send)]
impl MqttClient for AsyncClient {
	type Message = paho_mqtt::Message;
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

		AsyncClient::disconnect(self, builder.finalize())
			.await
			.into_report()
			.map(|_| ())
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
		paho_mqtt::MessageBuilder::retain(self, retain)
	}

	fn build(self) -> error_stack::Result<Self::Message, Self::Error> {
		Ok(self.finalize())
	}
}

// fn as_create_options(&self) -> error_stack::Result<paho_mqtt::CreateOptions, MqttOptionsError> {
// 	let builder = paho_mqtt::CreateOptionsBuilder::new()
// 		.client_id(format!("{}_{}", self.application_name, self.node_id))
// 		.server_uri(format!("{}:{}", self.mqtt.host, self.mqtt.port))
// 		.send_while_disconnected(true);

// 	let persistence_file = match &self.mqtt.persitence {
// 		MqttPersistence::Default => state_dir()
// 			.or_else(cache_dir)
// 			.map(|dir| self.join_persistence_file(&dir))
// 			.ok_or(MqttOptionsError::StateDir)?,
// 		MqttPersistence::File(d) => d.clone(),
// 		MqttPersistence::Directory(d) => self.join_persistence_file(d),
// 	};

// 	let builder = builder.persistence(persistence_file);

// 	Ok(builder.finalize())
// }

// pub(crate) async fn create_client(
// 	&self,
// ) -> error_stack::Result<HassMqttConnection, MqttOptionsError> {
// 	let node_id = NodeId::new(&*self.node_id);
// 	let discovery_topic = DiscoveryTopicConfig::new(&*self.discovery_prefix, node_id.clone());
// 	let private_topic = PrivateTopicConfig::new(&*self.private_prefix, node_id);

// 	let client = paho_mqtt::AsyncClient::new(self.as_create_options()?)
// 		.into_report()
// 		.change_context(MqttOptionsError::Client)?;

// 	let mut builder = paho_mqtt::ConnectOptionsBuilder::new();
// 	let hosts = lookup_host((&*self.mqtt.host, self.mqtt.port))
// 		.await
// 		.into_report()
// 		.change_context(MqttOptionsError::resolve_host(
// 			&self.mqtt.host,
// 			self.mqtt.port,
// 		))?
// 		.map(|addr| format!("tcp://{addr}"))
// 		.collect::<Vec<_>>();

// 	builder
// 		.server_uris(&hosts)
// 		.automatic_reconnect(Duration::from_secs(5), Duration::from_secs(60 * 5));

// 	let availability_topic = private_topic.node_topic("available");
// 	let will_message = availability_message(&availability_topic, "offline");
// 	let online_message = availability_message(&availability_topic, "online");

// 	builder.will_message(will_message);
// 	if self.mqtt.tls {
// 		builder.ssl_options(paho_mqtt::SslOptions::new());
// 	}

// 	if let Some(auth) = &self.mqtt.auth {
// 		builder.user_name(auth.username.clone());
// 		builder.password(auth.password.clone());
// 	}

// 	client.set_connected_callback(move |c| {
// 		// TODO: log
// 		let _ = c.publish(online_message.clone()).wait();
// 	});

// 	client
// 		.connect(builder.finalize())
// 		.await
// 		.into_report()
// 		.change_context(MqttOptionsError::Connect)?;

// 	Ok(HassMqttConnection {
// 		discovery: discovery_topic,
// 		private: private_topic,
// 		client,
// 	})
// }
