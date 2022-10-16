use crate::{
  exts::ValidateContextExt,
  entity::{Entity, EntityInvalidity},
  device_class::DeviceClass, payload::Payload, template::Template, topic::Topic,
};
use semval::{context::Context, Validate, ValidationResult};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

/// The mqtt binary sensor platform uses an MQTT message received to set the binary sensor’s
/// state to `on`, `off` or `unknown`.
///
/// The state will be updated only after a new message is published on `state_topic`
/// matching `payload_on`, `payload_off` or `None`.
/// If these messages are published with the `retain` flag set, the binary sensor will receive an
/// instant state update after subscription and Home Assistant will display the correct state on
/// startup. Otherwise, the initial state displayed in Home Assistant will be `unknown`.
///
/// Stateless devices such as buttons, remote controls etc are better represented by
/// [MQTT device triggers][device_trigger] than by binary sensors.
///
/// See: <https://www.home-assistant.io/integrations/binary_sensor.mqtt/>
///
/// [device_trigger]: https://www.home-assistant.io/integrations/device_trigger.mqtt/
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinarySensor<'a> {
  #[serde(borrow, flatten)]
  pub entity: Entity<'a>,

  /// The [type/class][device_class] of the sensor to set
  /// the icon in the frontend.
  ///
  /// [device_class]: https://www.home-assistant.io/integrations/binary_sensor/#device-class
  #[serde(default, skip_serializing_if = "DeviceClass::is_none")]
  pub device_class: DeviceClass,

  /// Defines the number of seconds after the value expires if it's not updated. After
  /// expiry, the sensor’s state becomes `unavailable`.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub expire_after: Option<NonZeroU32>,

  /// Sends update events (which results in update of state object’s last_changed) even if the
  /// sensor’s state hasn’t changed. Useful if you want to have meaningful value graphs in history
  /// or want to create an automation that triggers on every incoming state message (not only when
  /// the sensor’s new state is different to the current one).
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub force_update: Option<bool>,

  /// For sensors that only send `on` state updates (like PIRs), this variable sets a
  /// delay in seconds after which the sensor’s state will be updated back to `off`.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub off_delay: Option<NonZeroU32>,

  /// The string that represents the `off` state. It will be compared to the message in the
  /// `state_topic` (see `value_template` for details)
  /// Defaults to `"OFF"`.
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub payload_off: Option<Payload<'a>>,

  /// The string that represents the `on` state. It will be compared to the message in the
  /// `state_topic` (see `value_template` for details)
  /// Defaults to `"ON"`.
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub payload_on: Option<Payload<'a>>,

  /// The MQTT topic subscribed to receive sensor values.
  #[serde(borrow)]
  pub state_topic: Topic<'a>,

  /// Defines a [template][template] that returns a string to be compared to
  /// `payload_on`/`payload_off` or an empty string, in which case the MQTT message will be removed.
  /// Available variables: `entity_id`.
  /// Remove this option when `payload_on` and `payload_off` are sufficient to match your payloads
  /// (i.e no pre-processing of original message is required).
  ///
  /// [template]: https://www.home-assistant.io/docs/configuration/templating/#processing-incoming-data
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub value_template: Option<Template<'a>>,
}

impl<'a> Validate for BinarySensor<'a> {
  type Invalidity = EntityInvalidity;

  fn validate(&self) -> ValidationResult<Self::Invalidity> {
    Context::new()
      .validate_with(&self.entity, |v| v)
      .validate_with_opt(&self.payload_on, EntityInvalidity::Payload)
      .validate_with_opt(&self.payload_off, EntityInvalidity::Payload)
      .validate_with(&self.state_topic, EntityInvalidity::Topic)
      .validate_with_opt(&self.value_template, EntityInvalidity::Template)
      .into()
  }
}
