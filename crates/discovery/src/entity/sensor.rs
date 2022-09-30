use crate::{
  exts::ValidateContextExt,
  entity::{Entity, EntityInvalidity},
  device_class::DeviceClass, state_class::StateClass, template::Template, topic::Topic,
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
