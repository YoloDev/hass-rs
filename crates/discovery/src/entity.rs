use crate::{
  exts::ValidateContextExt,
  availability::{Availability, AvailabilityMode, AvailabilityDataInvalidity},
  device::{Device, DeviceInvalidity},
  icon::{Icon, IconInvalidity},
  name::{Name, NameInvalidity},
  payload::PayloadInvalidity,
  template::{Template, TemplateInvalidity},
  topic::{Topic, TopicInvalidity},
  entity_category::EntityCategory, qos::MqttQoS,
  unique_id::{UniqueId, UniqueIdInvalidity},
};
use semval::{context::Context, Validate, ValidationResult};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

mod binary_sensor;
mod button;
mod device_tracker;
mod sensor;
mod switch;

pub use binary_sensor::BinarySensor;
pub use button::Button;
pub use device_tracker::DeviceTracker;
pub use sensor::Sensor;
pub use switch::Switch;

/// Discoverable MQTT device configuration.
///
/// See: <https://www.home-assistant.io/docs/mqtt/discovery/>
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entity<'a> {
  /// A list of MQTT topics subscribed to receive availability (online/offline) updates.
  #[serde(borrow, default, skip_serializing_if = "<[Availability]>::is_empty")]
  pub availability: Cow<'a, [Availability<'a>]>,

  /// When `availability` is configured, this controls the conditions needed
  /// to set the entity to `available`.
  #[serde(default, skip_serializing_if = "AvailabilityMode::is_default")]
  pub availability_mode: AvailabilityMode,

  /// Information about the device this entity is a part of to tie it into the device registry.
  /// Only works through MQTT discovery and when `unique_id` is set.
  /// At least one of identifiers or connections must be present to identify the device.
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub device: Option<Device<'a>>,

  /// Flag which defines if the entity should be enabled when first added.
  /// Defaults to `true`.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub enabled_by_default: Option<bool>,

  /// The encoding of the payloads received and published messages. Set to "" to disable decoding of incoming payload.
  /// Defaults to `"utf-8"`.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub encoding: Option<Cow<'a, str>>,

  /// The [category] of the entity.
  ///
  /// [category]: https://developers.home-assistant.io/docs/core/entity#generic-properties
  #[serde(default, skip_serializing_if = "EntityCategory::is_none")]
  pub entity_category: EntityCategory,

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

  /// The MQTT topic subscribed to receive a JSON dictionary payload and then set as entity
  /// attributes.
  ///
  /// Implies `force_update` of the current state when a message is received on this topic.
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub json_attributes_topic: Option<Topic<'a>>,

  /// The name of the MQTT entity.
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub name: Option<Name<'a>>,

  /// Used instead of `name` for automatic generation of `entity_id`.
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub object_id: Option<Cow<'a, str>>,

  /// The maximum QoS level of the state topic.
  #[serde(default, skip_serializing_if = "MqttQoS::is_default")]
  pub qos: MqttQoS,

  /// An ID that uniquely identifies this entity. If two entities have the same unique ID,
  /// Home Assistant will raise an exception.
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub unique_id: Option<UniqueId<'a>>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum EntityInvalidity {
  Availability(usize, AvailabilityDataInvalidity),
  Device(DeviceInvalidity),
  Icon(IconInvalidity),
  Name(NameInvalidity),
  Payload(PayloadInvalidity),
  Template(TemplateInvalidity),
  Topic(TopicInvalidity),
  UniqueId(UniqueIdInvalidity),
}

impl<'a> Validate for Entity<'a> {
  type Invalidity = EntityInvalidity;

  fn validate(&self) -> ValidationResult<Self::Invalidity> {
    Context::new()
      .validate_iter(&*self.availability, EntityInvalidity::Availability)
      .validate_with_opt(&self.device, EntityInvalidity::Device)
      .validate_with_opt(&self.icon, EntityInvalidity::Icon)
      .validate_with_opt(&self.json_attributes_template, EntityInvalidity::Template)
      .validate_with_opt(&self.json_attributes_topic, EntityInvalidity::Topic)
      .validate_with_opt(&self.name, EntityInvalidity::Name)
      .validate_with_opt(&self.unique_id, EntityInvalidity::UniqueId)
      .into()
  }
}
