use crate::topics::ApplicationName;
use std::{path::PathBuf, sync::Arc};

#[derive(Clone)]
pub struct HassMqttOptions {
	pub(crate) mqtt: MqttOptions,
	pub(crate) discovery_prefix: String,
	pub(crate) private_prefix: Option<String>,
	pub(crate) application_name: ApplicationName,
	pub(crate) node_id: String,
}

impl HassMqttOptions {
	const DEFAULT_DISCOVERY_PREFIX: &'static str = "homeassistant";
	const DEFAULT_NODE_ID: &'static str = "default";

	pub fn new(host: impl Into<String>, application_name: impl Into<Arc<str>>) -> Self {
		let application_name = ApplicationName::new(application_name);

		HassMqttOptions {
			mqtt: MqttOptions::new(host),
			discovery_prefix: Self::DEFAULT_DISCOVERY_PREFIX.into(),
			private_prefix: None,
			application_name,
			node_id: Self::DEFAULT_NODE_ID.into(),
		}
	}

	#[cfg(feature = "tls")]
	#[cfg_attr(doc_cfg, doc(cfg(feature = "tls")))]
	pub fn new_tls(host: impl Into<String>, application_name: impl Into<Arc<str>>) -> Self {
		let application_name = ApplicationName::new(application_name);

		HassMqttOptions {
			mqtt: MqttOptions::new_tls(host),
			discovery_prefix: Self::DEFAULT_DISCOVERY_PREFIX.into(),
			private_prefix: None,
			application_name,
			node_id: Self::DEFAULT_NODE_ID.into(),
		}
	}

	pub fn port(mut self, port: u16) -> Self {
		self.mqtt.port(port);
		self
	}

	#[cfg(feature = "tls")]
	#[cfg_attr(doc_cfg, doc(cfg(feature = "tls")))]
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
		self.private_prefix = Some(private_prefix.into());
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
}

#[derive(Clone, Debug)]
pub(crate) enum MqttPersistence {
	Default,
	Directory(PathBuf),
	File(PathBuf),
}

#[derive(Clone)]
pub struct MqttOptions {
	pub(crate) host: String,
	pub(crate) port: u16,
	#[cfg(feature = "tls")]
	pub(crate) tls: bool,
	pub(crate) auth: Option<MqttAuthOptions>,
	pub(crate) persitence: MqttPersistence,
}

impl MqttOptions {
	pub fn new(host: impl Into<String>) -> Self {
		MqttOptions {
			host: host.into(),
			port: 1883,
			#[cfg(feature = "tls")]
			tls: false,
			auth: None,
			persitence: MqttPersistence::Default,
		}
	}

	#[cfg(feature = "tls")]
	#[cfg_attr(doc_cfg, doc(cfg(feature = "tls")))]
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

	#[cfg(feature = "tls")]
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

#[derive(Clone)]
pub(crate) struct MqttAuthOptions {
	pub(crate) username: String,
	pub(crate) password: String,
}
