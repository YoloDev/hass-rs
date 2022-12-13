use crate::topics::{ApplicationName, NodeId};
use dirs::{cache_dir, state_dir};
use hass_dyn_error::DynError;
use std::{
	backtrace::Backtrace,
	fmt,
	path::{Path, PathBuf},
	sync::Arc,
};
use thiserror::Error;
#[cfg(feature = "spantrace")]
use tracing_error::SpanTrace;

#[derive(Clone)]
pub struct HassMqttOptions {
	pub(crate) mqtt: MqttOptions,
	pub(crate) discovery_prefix: String,
	pub(crate) private_prefix: Option<String>,
	pub(crate) application_name: ApplicationName,
	pub(crate) node_id: NodeId,
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
		self.node_id = NodeId::new(node_id.into());
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

#[derive(Debug)]
pub struct MqttPersistenceError {
	#[cfg(feature = "backtrace")]
	backtrace: Backtrace,
	#[cfg(feature = "spantrace")]
	spantrace: SpanTrace,
}

impl MqttPersistenceError {
	pub fn new() -> Self {
		MqttPersistenceError {
			#[cfg(feature = "backtrace")]
			backtrace: Backtrace::capture(),
			#[cfg(feature = "spantrace")]
			spantrace: SpanTrace::capture(),
		}
	}

	#[cfg(feature = "backtrace")]
	pub fn backtrace(&self) -> &Backtrace {
		&self.backtrace
	}

	#[cfg(feature = "spantrace")]
	pub fn spantrace(&self) -> &SpanTrace {
		&self.spantrace
	}
}

impl Default for MqttPersistenceError {
	fn default() -> Self {
		Self::new()
	}
}

impl fmt::Display for MqttPersistenceError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("failed to get state dir")
	}
}

impl std::error::Error for MqttPersistenceError {
	#[cfg(provide_any)]
	fn provide<'a>(&'a self, demand: &mut std::any::Demand<'a>) {
		#[cfg(feature = "backtrace")]
		demand.provide_ref(&self.backtrace);
		#[cfg(feature = "spantrace")]
		demand.provide_ref(&self.spantrace);
	}
}

#[derive(Clone, Debug)]
pub(crate) enum MqttPersistence {
	Default,
	Directory(PathBuf),
	File(PathBuf),
}

impl MqttPersistence {
	fn to_path(
		&self,
		application_name: &ApplicationName,
		node_id: &NodeId,
	) -> Result<PathBuf, MqttPersistenceError> {
		fn join_persistence_file(
			dir: &Path,
			application_name: &ApplicationName,
			node_id: &NodeId,
		) -> PathBuf {
			dir.join(format!("{}.{}.mqtt", application_name.slug(), node_id))
		}

		match self {
			MqttPersistence::Default => state_dir()
				.or_else(cache_dir)
				.map(|dir| join_persistence_file(&dir, application_name, node_id))
				.ok_or_else(MqttPersistenceError::new),
			MqttPersistence::File(d) => Ok(d.clone()),
			MqttPersistence::Directory(d) => Ok(join_persistence_file(d, application_name, node_id)),
		}
	}
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

#[derive(Debug, Error)]
#[error("failed to convert ot mqtt options")]
pub struct MqttOptionsError {
	#[cfg_attr(provide_any, backtrace)]
	source: DynError,
}

impl MqttOptionsError {
	pub(crate) fn new(source: impl std::error::Error + Send + Sync + 'static) -> Self {
		MqttOptionsError {
			source: DynError::new(source),
		}
	}
}

impl TryInto<hass_mqtt_provider::MqttOptions> for HassMqttOptions {
	type Error = MqttOptionsError;

	fn try_into(self) -> Result<hass_mqtt_provider::MqttOptions, Self::Error> {
		let persistence = self
			.mqtt
			.persitence
			.to_path(&self.application_name, &self.node_id)
			.map_err(MqttOptionsError::new)?;

		let mut options = hass_mqtt_provider::MqttOptions::new(self.mqtt.host, persistence);
		options.port(self.mqtt.port);

		#[cfg(feature = "tls")]
		options.tls(self.mqtt.tls);

		if let Some(auth) = self.mqtt.auth {
			options.auth(auth.username, auth.password);
		}

		Ok(options)
	}
}
