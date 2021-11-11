use slug::slugify;

#[derive(Clone)]
pub struct HassMqttOptions {
  pub(crate) mqtt: MqttOptions,
  pub(crate) discovery_prefix: String,
  pub(crate) private_prefix: String,
  pub(crate) node_id: Option<String>,
}

impl HassMqttOptions {
  pub fn new(host: impl Into<String>, application_name: impl Into<String>) -> Self {
    HassMqttOptions {
      mqtt: MqttOptions::new(host),
      discovery_prefix: "homeassistant".into(),
      private_prefix: slugify(application_name.into()),
      node_id: None,
    }
  }

  pub fn new_tls(host: impl Into<String>, application_name: impl Into<String>) -> Self {
    HassMqttOptions {
      mqtt: MqttOptions::new_tls(host),
      discovery_prefix: "homeassistant".into(),
      private_prefix: slugify(application_name.into()),
      node_id: None,
    }
  }

  pub fn port(&mut self, port: u16) -> &mut Self {
    self.mqtt.port(port);
    self
  }

  pub fn tls(&mut self, tls: bool) -> &mut Self {
    self.mqtt.tls(tls);
    self
  }

  pub fn auth(&mut self, username: impl Into<String>, password: impl Into<String>) -> &mut Self {
    self.mqtt.auth(username, password);
    self
  }

  pub fn discovery_prefix(&mut self, discovery_prefix: impl Into<String>) -> &mut Self {
    self.discovery_prefix = discovery_prefix.into();
    self
  }

  pub fn private_prefix(&mut self, private_prefix: impl Into<String>) -> &mut Self {
    self.private_prefix = private_prefix.into();
    self
  }

  pub fn node_id(&mut self, node_id: impl Into<String>) -> &mut Self {
    self.node_id = Some(node_id.into());
    self
  }
}

#[derive(Clone)]
pub(crate) struct MqttOptions {
  pub(crate) host: String,
  pub(crate) port: u16,
  pub(crate) tls: bool,
  pub(crate) auth: Option<MqttAuthOptions>,
}

impl MqttOptions {
  pub fn new(host: impl Into<String>) -> Self {
    MqttOptions {
      host: host.into(),
      port: 1883,
      tls: false,
      auth: None,
    }
  }

  pub fn new_tls(host: impl Into<String>) -> Self {
    MqttOptions {
      host: host.into(),
      port: 8883,
      tls: true,
      auth: None,
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
}

#[derive(Clone)]
pub(crate) struct MqttAuthOptions {
  pub(crate) username: String,
  pub(crate) password: String,
}
