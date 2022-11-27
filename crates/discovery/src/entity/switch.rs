use crate::{device_class::DeviceClass, payload::Payload, template::Template, topic::Topic};
use hass_mqtt_discovery_macros::entity_document;

/// The mqtt switch platform lets you control your MQTT enabled switches.
///
/// See: <https://www.home-assistant.io/integrations/switch.mqtt/>
#[entity_document]
pub struct Switch<'a> {
  /// The MQTT topic to publish commands to change the switch state.
  #[serde(borrow)]
  pub command_topic: Topic<'a>,

  /// The [type/class][device_class] of the switch to set the icon in the frontend.
  ///
  /// [device_class]: https://www.home-assistant.io/integrations/switch/#device-class
  #[serde(default, skip_serializing_if = "DeviceClass::is_none")]
  pub device_class: DeviceClass,

  /// Flag that defines if switch works in optimistic mode.
  /// Defaults to `true` if no `state_topic` defined, else `false`.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub optimistic: Option<bool>,

  /// The payload that represents `off` state. If specified, will be
  /// used for both comparing to the value in the `state_topic` (see
  /// `value_template` and `state_off` for details) and sending as
  /// `off` command to the `command_topic`.
  /// Defaults to `"OFF"`.
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub payload_off: Option<Payload<'a>>,

  /// The payload that represents `on` state. If specified, will be
  /// used for both comparing to the value in the `state_topic` (see
  /// `value_template` and `state_on` for details) and sending as
  /// `on` command to the `command_topic`.
  /// Defaults to `"ON"`.
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub payload_on: Option<Payload<'a>>,

  /// If the published message should have the retain flag on or not.
  /// Defaults to `false`.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub retain: Option<bool>,

  /// The payload that represents the `off` state. Used when value that
  /// represents `off` state in the `state_topic` is different from value that
  /// should be sent to the `command_topic` to turn the device `off`.
  /// Defaults to `payload_off` if defined, else `"OFF"`.
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub state_off: Option<Payload<'a>>,

  /// The payload that represents the `on` state. Used when value that
  /// represents on state in the `state_topic` is different from value that
  /// should be sent to the `command_topic` to turn the device `on`.
  /// Defaults to `payload_on` if defined, else `"ON"`.
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub state_on: Option<Payload<'a>>,

  /// The MQTT topic subscribed to receive state updates.
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub state_topic: Option<Topic<'a>>,

  /// Defines a [template][template] to extract device’s state from the
  /// `state_topic`. To determine the switches’s state result of this
  /// template will be compared to `state_on` and `state_off`.
  ///
  /// [template]: https://www.home-assistant.io/docs/configuration/templating/#processing-incoming-data
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub value_template: Option<Template<'a>>,
}
