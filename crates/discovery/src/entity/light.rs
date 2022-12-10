use crate::{topic::Topic, validation::Validator};
use enumset::{EnumSet, EnumSetType};
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
	#[entity(validate = "ColorModeSetValidator")]
	#[serde(default, skip_serializing_if = "EnumSet::is_empty")]
	pub supported_color_modes: EnumSet<ColorMode>,

	/// Defines the maximum white level (i.e., 100%) of the MQTT device. This
	/// is used when setting the light to white mode.
	/// Defaults to `255`.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub white_value_scale: Option<u8>,
}

impl<'a> Validator for Light<'a> {
	type Invalidity = LightInvalidity;

	fn validate_value(
		&self,
		value: &Self,
		context: semval::context::Context<Self::Invalidity>,
	) -> semval::context::Context<Self::Invalidity> {
		context.invalidate_if(
			value.color_mode == Some(true) && value.supported_color_modes.is_empty(),
			LightInvalidity::ColorModeWithoutSupportedColorModes,
		)
	}
}

/// Color modes for lights.
#[derive(EnumSetType, Debug, serde::Serialize, serde::Deserialize)]
#[enumset(serialize_as_list)]
pub enum ColorMode {
	/// The light can be turned on or off. This mode must be the
	/// only supported mode if supported by the light.
	#[serde(rename = "onoff")]
	OnOff,

	/// The light can be dimmed. This mode must be the only supported
	/// mode if supported by the light.
	#[serde(rename = "brightness")]
	Brightness,

	/// The light can be dimmed and its color temperature is present
	/// in the state.
	#[serde(rename = "color_temp")]
	ColorTemp,

	/// The light can be dimmed and its color can be adjusted. The light's
	/// brightness can be set using the [LightState::brightness] parameter
	/// and read through the [LightState::brightness] property. The light's
	/// color can be set using the [LightColorState::hue] and
	/// [LightColorState::saturation] parameter and read using the same
	/// properties (not normalized for brightness).
	#[serde(rename = "hs")]
	HueSaturation,

	/// The light can be dimmed and its color can be adjusted. The light's
	/// brightness can be set using the [LightState::brightness] parameter
	/// and read through the [LightState::brightness] property. The light's
	/// color can be set using the [LightColorState::red], [LightColorState::green],
	/// and [LightColorState::blue] parameter and read using the same
	/// properties (not normalized for brightness).
	#[serde(rename = "rgb")]
	RedGreenBlue,

	/// The light can be dimmed and its color can be adjusted. The light's
	/// brightness can be set using the [LightState::brightness] parameter
	/// and read through the [LightState::brightness] property. The light's
	/// color can be set using the [LightColorState::red], [LightColorState::green],
	/// [LightColorState::blue], and [LightColorState::white] parameter and
	/// read using the same properties (not normalized for brightness).
	#[serde(rename = "rgbw")]
	RedGreenBlueWhite,

	/// The light can be dimmed and its color can be adjusted. The light's
	/// brightness can be set using the [LightState::brightness] parameter
	/// and read through the [LightState::brightness] property. The light's
	/// color can be set using the [LightColorState::red], [LightColorState::green],
	/// [LightColorState::blue], [LightColorState::cold_white], and
	/// [LightColorState::warm_white] parameter and read using the same
	/// properties (not normalized for brightness).
	#[serde(rename = "rgbww")]
	RedGreenBlueWhiteWarmWhite,

	/// The light can be dimmed and its color can be adjusted. In addition,
	/// the light can be set to white mode. The light's brightness can be
	/// set using the [LightState::brightness] parameter and read through
	/// the [LightState::brightness] property. If this mode is supported, the
	/// light must also support at least one of [ColorMode::HueSaturation],
	/// [ColorMode::RedGreenBlue], [ColorMode::RedGreenBlueWhite],
	/// [ColorMode::RedGreenBlueWhiteWarmWhite] or [ColorMode::XY].
	#[serde(rename = "white")]
	White,

	/// The light can be dimmed and its color can be adjusted. The light's
	/// brightness can be set using the [LightState::brightness] parameter
	/// and read through the [LightState::brightness] property. The light's
	/// color can be set using the [LightColorState::x] and [LightColorState::y]
	/// parameter and read using the same properties (not normalized for brightness).
	#[serde(rename = "xy")]
	XY,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorModesInvalidity {
	OnOffWithOthers,
	BrightnessWithOthers,
	WhiteWithoutColorModes,
}

pub struct ColorModeSetValidator;

impl Validator<EnumSet<ColorMode>> for ColorModeSetValidator {
	type Invalidity = ColorModesInvalidity;

	fn validate_value(
		&self,
		value: &EnumSet<ColorMode>,
		context: semval::context::Context<ColorModesInvalidity>,
	) -> semval::context::Context<ColorModesInvalidity> {
		let value = *value;
		context
			.invalidate_if(
				value.contains(ColorMode::OnOff) && value != ColorMode::OnOff,
				ColorModesInvalidity::OnOffWithOthers,
			)
			.invalidate_if(
				value.contains(ColorMode::Brightness) && value != ColorMode::Brightness,
				ColorModesInvalidity::BrightnessWithOthers,
			)
			.invalidate_if(
				value.contains(ColorMode::White)
					&& value.is_disjoint(
						ColorMode::HueSaturation
							| ColorMode::RedGreenBlue
							| ColorMode::RedGreenBlueWhite
							| ColorMode::RedGreenBlueWhiteWarmWhite
							| ColorMode::XY,
					),
				ColorModesInvalidity::WhiteWithoutColorModes,
			)
	}
}
