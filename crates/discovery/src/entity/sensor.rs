use crate::{
  device::Device,
  device_class::DeviceClass, entity_category::EntityCategory, icon::Icon, name::Name, qos::MqttQoS,
  state_class::StateClass, template::Template, topic::Topic, unique_id::UniqueId,
};
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
  /// Information about the device this sensor is a part of to tie it into the device registry.
  /// Only works through MQTT discovery and when `unique_id` is set.
  /// At least one of identifiers or connections must be present to identify the device.
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub device: Option<Device<'a>>,

  /// The [type/class][device_class] of the sensor to set
  /// the icon in the frontend.
  ///
  /// [device_class]: https://www.home-assistant.io/integrations/sensor/#device-class
  #[serde(default, skip_serializing_if = "DeviceClass::is_none")]
  pub device_class: DeviceClass,

  /// Flag which defines if the entity should be enabled when first added.
  /// Defaults to `true`.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub enabled_by_default: Option<bool>,

  /// The [category] of the entity.
  ///
  /// [category]: https://developers.home-assistant.io/docs/core/entity#generic-properties
  #[serde(default, skip_serializing_if = "EntityCategory::is_none")]
  pub entity_category: EntityCategory,

  /// Defines the number of seconds after the value expires if it's not updated. After
  /// expiry, the sensor’s state becomes `unavailable`.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub expire_after: Option<NonZeroU32>,

  /// Sends update events even if the value hasn’t changed. Useful if you want to have
  /// meaningful value graphs in history. Defaults to `false`.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub force_update: Option<bool>,

  /// [Icon][icon] for the entity.
  ///
  /// [icon]: https://www.home-assistant.io/docs/configuration/customizing-devices/#icon
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub icon: Option<Icon<'a>>,

  /// Defines a [template][template] to extract the JSON dictionary from messages received
  /// on the `json_attributes_topic`.
  ///
  /// [template]: https://www.home-assistant.io/docs/configuration/templating/#processing-incoming-data
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub json_attributes_template: Option<Template<'a>>,

  /// The MQTT topic subscribed to receive a JSON dictionary payload and then set as sensor
  /// attributes. Implies `force_update` of the current sensor state when a message is
  /// received on this topic.
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub json_attributes_topic: Option<Topic<'a>>,

  /// Defines a [template][template] to extract the last_reset. Available variables: `entity_id`.
  /// The `entity_id` can be used to reference the entity’s attributes.
  ///
  /// [template]: https://www.home-assistant.io/docs/configuration/templating/#processing-incoming-data
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub last_reset_value_template: Option<Template<'a>>,

  /// The name of the MQTT sensor. Default: `MQTT Sensor`.
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub name: Option<Name<'a>>,

  /// Used instead of `name` for automatic generation of `entity_id`.
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub object_id: Option<Cow<'a, str>>,

  /// The maximum QoS level of the state topic.
  #[serde(default, skip_serializing_if = "MqttQoS::is_default")]
  pub qos: MqttQoS,

  /// The [state_class][state_class] of the sensor.
  ///
  /// [state_class]: https://developers.home-assistant.io/docs/core/entity/sensor#available-state-classes
  #[serde(default, skip_serializing_if = "StateClass::is_none")]
  pub state_class: StateClass,

  /// The MQTT topic subscribed to receive sensor values.
  #[serde(borrow)]
  pub state_topic: Topic<'a>,

  /// An ID that uniquely identifies this sensor. If two sensors have the same unique ID,
  /// Home Assistant will raise an exception.
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub unique_id: Option<UniqueId<'a>>,

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
