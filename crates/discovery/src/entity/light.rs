use crate::{topic::Topic, validation::CustomValidation};
use hass_mqtt_discovery_macros::entity_document;
use std::borrow::Cow;

/// The mqtt light platform lets you control your MQTT enabled lights.
///
/// See: <https://www.home-assistant.io/integrations/light.mqtt/#json-schema>
#[entity_document]
#[entity(extend_json(schema = "json"))]
#[entity(validate(ColorModeWithoutSupportedColorModes))]
pub struct Light<'a> {
  /// Flag that defines if the light supports brightness.
  /// Defaults to `false`.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub brightness: Option<bool>,

  /// Defines the maximum brightness value (i.e., 100%) of the MQTT device.
  /// Defaults to `255`.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub brightness_scale: Option<u8>,

  /// Flag that defines if the light supports color modes.
  /// Defaults to `false`.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub color_mode: Option<bool>,

  /// The MQTT topic to publish commands to change the light’s state.
  #[serde(borrow)]
  pub command_topic: Topic<'a>,

  /// Flag that defines if the light supports effects.
  /// Defaults to `false`.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub effect: Option<bool>,

  /// The list of effects the light supports.
  #[serde(borrow, default, skip_serializing_if = "<[Cow<str>]>::is_empty")]
  pub effect_list: Cow<'a, [Cow<'a, str>]>,

  /// The duration, in seconds, of a “long” flash.
  /// Defaults to `10`.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub flash_time_long: Option<u8>,

  /// The duration, in seconds, of a “short” flash.
  /// Defaults to `2`.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub flash_time_short: Option<u8>,

  /// The maximum color temperature in mireds.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub max_mireds: Option<u16>,

  /// The minimum color temperature in mireds.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub min_mireds: Option<u16>,

  /// Flag that defines if light works in optimistic mode.
  /// Defaults to `true` if no `state_topic` defined, else `false`.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub optimistic: Option<bool>,

  /// If the published message should have the retain flag on or not.
  /// Defaults to `false`.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub retain: Option<bool>,

  /// The MQTT topic subscribed to receive state updates.
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub state_topic: Option<Topic<'a>>,

  /// A list of color modes supported by the list. This is required if
  /// [color_mode] is `true`.
  #[serde(borrow, default, skip_serializing_if = "<[ColorMode]>::is_empty")]
  pub supported_color_modes: Cow<'a, [ColorMode]>,

  /// Defines the maximum white level (i.e., 100%) of the MQTT device. This
  /// is used when setting the light to white mode.
  /// Defaults to `255`.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub white_value_scale: Option<u8>,
}

impl<'a> CustomValidation for Light<'a> {
  type Invalidity = LightInvalidity;

  fn additional_validation(
    &self,
    context: semval::context::Context<Self::Invalidity>,
  ) -> semval::context::Context<Self::Invalidity> {
    context.invalidate_if(
      self.color_mode == Some(true) && self.supported_color_modes.is_empty(),
      LightInvalidity::ColorModeWithoutSupportedColorModes,
    )
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ColorMode {
  #[serde(rename = "onoff")]
  OnOff,

  #[serde(rename = "brightness")]
  Brightness,

  #[serde(rename = "color_temp")]
  ColorTemp,

  #[serde(rename = "hs")]
  Hs,

  #[serde(rename = "xy")]
  Xy,

  #[serde(rename = "rgb")]
  Rgb,

  #[serde(rename = "rgbw")]
  Rgbw,

  #[serde(rename = "rgbww")]
  Rgbww,

  #[serde(rename = "white")]
  White,
}
