use crate::{
  device_class::DeviceClass,
  entity::{Entity, EntityInvalidity},
  exts::ValidateContextExt,
  state_class::StateClass,
  template::Template,
  topic::Topic,
};
use semval::{context::Context, Validate, ValidationResult};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, num::NonZeroU32};

/// This mqtt sensor platform uses the MQTT message payload as the sensor value.
/// If messages in this state_topic are published with RETAIN flag, the sensor
/// will receive an instant update with last known value. Otherwise, the initial
/// state will be undefined.
///
/// See: <https://www.home-assistant.io/integrations/sensor.mqtt/>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sensor<'a> {
  #[serde(borrow, flatten)]
  pub entity: Entity<'a>,

  /// The [type/class][device_class] of the sensor to set
  /// the icon in the frontend.
  ///
  /// [device_class]: https://www.home-assistant.io/integrations/sensor/#device-class
  #[serde(default, skip_serializing_if = "DeviceClass::is_none")]
  pub device_class: DeviceClass,

  /// Defines the number of seconds after the value expires if it's not updated. After
  /// expiry, the sensor’s state becomes `unavailable`.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub expire_after: Option<NonZeroU32>,

  /// Sends update events even if the value hasn’t changed. Useful if you want to have
  /// meaningful value graphs in history. Defaults to `false`.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub force_update: Option<bool>,

  /// Defines a [template][template] to extract the last_reset. Available variables: `entity_id`.
  /// The `entity_id` can be used to reference the entity’s attributes.
  ///
  /// [template]: https://www.home-assistant.io/docs/configuration/templating/#processing-incoming-data
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub last_reset_value_template: Option<Template<'a>>,

  /// The [state_class][state_class] of the sensor.
  ///
  /// [state_class]: https://developers.home-assistant.io/docs/core/entity/sensor#available-state-classes
  #[serde(default, skip_serializing_if = "StateClass::is_none")]
  pub state_class: StateClass,

  /// The MQTT topic subscribed to receive sensor values.
  #[serde(borrow)]
  pub state_topic: Topic<'a>,

  /// Defines the units of measurement of the sensor, if any.
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub unit_of_measurement: Option<Cow<'a, str>>,

  /// Defines a [template][template] to extract the value. Available variables: `entity_id`.
  /// The `entity_id` can be used to reference the entity’s attributes.
  ///
  /// [template]: https://www.home-assistant.io/docs/configuration/templating/#processing-incoming-data
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub value_template: Option<Template<'a>>,
}

impl<'a> Validate for Sensor<'a> {
  type Invalidity = EntityInvalidity;

  fn validate(&self) -> ValidationResult<Self::Invalidity> {
    Context::new()
      .validate_with(&self.entity, |v| v)
      .validate_with_opt(&self.last_reset_value_template, EntityInvalidity::Template)
      .validate_with(&self.state_topic, EntityInvalidity::Topic)
      .validate_with_opt(&self.value_template, EntityInvalidity::Template)
      .into()
  }
}
pub struct SensorBuilder<'a, T> {
  entity: Entity<'a>,
  device_class: DeviceClass,
  expire_after: Option<NonZeroU32>,
  force_update: Option<bool>,
  last_reset_value_template: Option<Template<'a>>,
  state_class: StateClass,
  state_topic: T,
  unit_of_measurement: Option<Cow<'a, str>>,
  value_template: Option<Template<'a>>,
}

impl<'a> Sensor<'a> {
  pub fn builder() -> SensorBuilder<'a, ()> {
    SensorBuilder {
      entity: Default::default(),
      device_class: Default::default(),
      expire_after: Default::default(),
      force_update: Default::default(),
      last_reset_value_template: Default::default(),
      state_class: Default::default(),
      state_topic: (),
      unit_of_measurement: Default::default(),
      value_template: Default::default(),
    }
  }
}

impl<'a, T> SensorBuilder<'a, T> {
  // TODO: Provide setters for all the fileds
  pub fn entity(mut self, entity: Entity<'a>) -> SensorBuilder<'a, T> {
    self.entity = entity;
    self
  }

  pub fn device_class(mut self, device_class: DeviceClass) -> SensorBuilder<'a, T> {
    self.device_class = device_class;
    self
  }

  pub fn expire_after(mut self, expire_after: Option<NonZeroU32>) -> SensorBuilder<'a, T> {
    self.expire_after = expire_after;
    self
  }

  pub fn force_update(mut self, force_update: Option<bool>) -> SensorBuilder<'a, T> {
    self.force_update = force_update;
    self
  }

  pub fn last_reset_value_template(
    mut self,
    last_reset_value_template: Option<Template<'a>>,
  ) -> SensorBuilder<'a, T> {
    self.last_reset_value_template = last_reset_value_template;
    self
  }

  pub fn state_class(mut self, state_class: StateClass) -> SensorBuilder<'a, T> {
    self.state_class = state_class;
    self
  }

  pub fn state_topic<U>(self, state_topic: U) -> SensorBuilder<'a, U>
  where
    U: Into<Topic<'a>>,
  {
    SensorBuilder {
      entity: self.entity,
      device_class: self.device_class,
      expire_after: self.expire_after,
      force_update: self.force_update,
      last_reset_value_template: self.last_reset_value_template,
      state_class: self.state_class,
      state_topic,
      unit_of_measurement: self.unit_of_measurement,
      value_template: self.value_template,
    }
  }

  pub fn unit_of_measurement(
    mut self,
    unit_of_measurement: Option<Cow<'a, str>>,
  ) -> SensorBuilder<'a, T> {
    self.unit_of_measurement = unit_of_measurement;
    self
  }

  pub fn value_template(mut self, value_template: Option<Template<'a>>) -> SensorBuilder<'a, T> {
    self.value_template = value_template;
    self
  }
}

impl<'a, T> SensorBuilder<'a, T>
where
  T: Into<Topic<'a>>,
{
  pub fn build(self) -> Sensor<'a> {
    Sensor {
      entity: self.entity,
      device_class: self.device_class,
      expire_after: self.expire_after,
      force_update: self.force_update,
      last_reset_value_template: self.last_reset_value_template,
      state_class: self.state_class,
      state_topic: self.state_topic.into(),
      unit_of_measurement: self.unit_of_measurement,
      value_template: self.value_template,
    }
  }
}
