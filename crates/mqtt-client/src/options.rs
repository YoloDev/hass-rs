use crate::topics::{DiscoveryTopicConfig, NodeId, PrivateTopicConfig};
use dirs::{cache_dir, state_dir};
use error_stack::{IntoReport, ResultExt};
use slug::slugify;
use std::{
	path::{Path, PathBuf},
	time::Duration,
};
use thiserror::Error;
use tokio::net::lookup_host;

pub(crate) struct HassMqttConnection {
	pub(crate) discovery: DiscoveryTopicConfig,
	pub(crate) private: PrivateTopicConfig,
	pub(crate) client: paho_mqtt::AsyncClient,
}

#[derive(Clone)]
pub struct HassMqttOptions {
	pub(crate) mqtt: MqttOptions,
	pub(crate) discovery_prefix: String,
	pub(crate) private_prefix: String,
	pub(crate) application_name: String,
	pub(crate) application_slug: String,
	pub(crate) node_id: String,
}

impl HassMqttOptions {
	const DEFAULT_DISCOVERY_PREFIX: &'static str = "homeassistant";
	const DEFAULT_NODE_ID: &'static str = "default";

	pub fn new(host: impl Into<String>, application_name: impl Into<String>) -> Self {
		let application_name = application_name.into();
		let application_slug = slugify(&application_name);

		HassMqttOptions {
			mqtt: MqttOptions::new(host),
			discovery_prefix: Self::DEFAULT_DISCOVERY_PREFIX.into(),
			private_prefix: application_slug.clone(),
			application_slug,
			application_name,
			node_id: Self::DEFAULT_NODE_ID.into(),
		}
	}

	pub fn new_tls(host: impl Into<String>, application_name: impl Into<String>) -> Self {
		let application_name = application_name.into();
		let application_slug = slugify(&application_name);

		HassMqttOptions {
			mqtt: MqttOptions::new_tls(host),
			discovery_prefix: Self::DEFAULT_DISCOVERY_PREFIX.into(),
			private_prefix: application_slug.clone(),
			application_slug,
			application_name,
			node_id: Self::DEFAULT_NODE_ID.into(),
		}
	}

	pub fn port(mut self, port: u16) -> Self {
		self.mqtt.port(port);
		self
	}

	pub fn tls(mut self, tls: bool) -> Self {
		self.mqtt.tls(tls);
		self
	}

	pub fn auth(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
		self.mqtt.auth(username, password);
		self
	}

	pub fn discovery_prefix(mut self, discovery_prefix: impl Into<String>) -> Self {
		self.discovery_prefix = discovery_prefix.into();
		self
	}

	pub fn private_prefix(mut self, private_prefix: impl Into<String>) -> Self {
		self.private_prefix = private_prefix.into();
		self
	}

	pub fn node_id(mut self, node_id: impl Into<String>) -> Self {
		self.node_id = node_id.into();
		self
	}

	pub fn persistence_dir(mut self, dir: impl Into<PathBuf>) -> Self {
		self.mqtt.persistence_dir(dir);
		self
	}

	pub fn persistence_file(mut self, file: impl Into<PathBuf>) -> Self {
		self.mqtt.persistence_file(file);
		self
	}

	fn join_persistence_file(&self, dir: &Path) -> PathBuf {
		dir.join(format!("{}_{}.mqtt", self.application_name, self.node_id))
	}

	fn as_create_options(&self) -> error_stack::Result<paho_mqtt::CreateOptions, MqttOptionsError> {
		let builder = paho_mqtt::CreateOptionsBuilder::new()
			.client_id(format!("{}_{}", self.application_name, self.node_id))
			.server_uri(format!("{}:{}", self.mqtt.host, self.mqtt.port))
			.send_while_disconnected(true);

		let persistence_file = match &self.mqtt.persitence {
			MqttPersistence::Default => state_dir()
				.or_else(cache_dir)
				.map(|dir| self.join_persistence_file(&dir))
				.ok_or(MqttOptionsError::StateDir)?,
			MqttPersistence::File(d) => d.clone(),
			MqttPersistence::Directory(d) => self.join_persistence_file(d),
		};

		let builder = builder.persistence(persistence_file);

		Ok(builder.finalize())
	}

	pub(crate) async fn create_client(
		&self,
	) -> error_stack::Result<HassMqttConnection, MqttOptionsError> {
		let node_id = NodeId::new(&*self.node_id);
		let discovery_topic = DiscoveryTopicConfig::new(&*self.discovery_prefix, node_id.clone());
		let private_topic = PrivateTopicConfig::new(&*self.private_prefix, node_id);

		let client = paho_mqtt::AsyncClient::new(self.as_create_options()?)
			.into_report()
			.change_context(MqttOptionsError::Client)?;

		let mut builder = paho_mqtt::ConnectOptionsBuilder::new();
		let hosts = lookup_host((&*self.mqtt.host, self.mqtt.port))
			.await
			.into_report()
			.change_context(MqttOptionsError::resolve_host(
				&self.mqtt.host,
				self.mqtt.port,
			))?
			.map(|addr| format!("tcp://{addr}"))
			.collect::<Vec<_>>();

		builder
			.server_uris(&hosts)
			.automatic_reconnect(Duration::from_secs(5), Duration::from_secs(60 * 5));

		let availability_topic = private_topic.node_topic("available");
		let will_message = availability_message(&availability_topic, "offline");
		let online_message = availability_message(&availability_topic, "online");

		builder.will_message(will_message);
		if self.mqtt.tls {
			builder.ssl_options(paho_mqtt::SslOptions::new());
		}

		if let Some(auth) = &self.mqtt.auth {
			builder.user_name(auth.username.clone());
			builder.password(auth.password.clone());
		}

		client.set_connected_callback(move |c| {
			// TODO: log
			let _ = c.publish(online_message.clone()).wait();
		});

		client
			.connect(builder.finalize())
			.await
			.into_report()
			.change_context(MqttOptionsError::Connect)?;

		Ok(HassMqttConnection {
			discovery: discovery_topic,
			private: private_topic,
			client,
		})
	}
}

#[derive(Clone, Debug)]
pub(crate) enum MqttPersistence {
	Default,
	Directory(PathBuf),
	File(PathBuf),
}

#[derive(Clone)]
pub(crate) struct MqttOptions {
	pub(crate) host: String,
	pub(crate) port: u16,
	pub(crate) tls: bool,
	pub(crate) auth: Option<MqttAuthOptions>,
	pub(crate) persitence: MqttPersistence,
}

impl MqttOptions {
	pub fn new(host: impl Into<String>) -> Self {
		MqttOptions {
			host: host.into(),
			port: 1883,
			tls: false,
			auth: None,
			persitence: MqttPersistence::Default,
		}
	}

	pub fn new_tls(host: impl Into<String>) -> Self {
		MqttOptions {
			host: host.into(),
			port: 8883,
			tls: true,
			auth: None,
			persitence: MqttPersistence::Default,
		}
	}

	pub fn port(&mut self, port: u16) -> &mut Self {
		self.port = port;
		self
	}

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

	fn persistence_dir(&mut self, dir: impl Into<PathBuf>) -> &mut Self {
		self.persitence = MqttPersistence::Directory(dir.into());
		self
	}

	fn persistence_file(&mut self, file: impl Into<PathBuf>) -> &mut Self {
		self.persitence = MqttPersistence::File(file.into());
		self
	}
}

#[derive(Debug, Clone, Error)]
pub(crate) enum MqttOptionsError {
	#[error("failed to create MQTT client")]
	Client,

	#[error("failed to connect to MQTT broker")]
	Connect,

	#[error("faild to find state or cache directory")]
	StateDir,

	#[error("falied to resolve host: {host}:{port}")]
	ResolveHost { host: String, port: u16 },
}

impl MqttOptionsError {
	fn resolve_host(host: &str, port: u16) -> Self {
		MqttOptionsError::ResolveHost {
			host: host.into(),
			port,
		}
	}
}

#[derive(Clone)]
pub(crate) struct MqttAuthOptions {
	pub(crate) username: String,
	pub(crate) password: String,
}

fn availability_message(topic: &str, content: &str) -> paho_mqtt::Message {
	paho_mqtt::MessageBuilder::default()
		.topic(topic)
		.payload(content)
		.qos(2)
		.retained(true)
		.finalize()
}
