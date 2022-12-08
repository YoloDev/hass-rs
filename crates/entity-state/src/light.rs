//! See [home-assistant light](https://developers.home-assistant.io/docs/core/entity/light/)
//! documentation for more info.

use enumset::EnumSetType;
use serde::{Deserialize, Serialize};

/// Color modes for lights.
#[derive(EnumSetType, Debug, Serialize, Deserialize)]
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
