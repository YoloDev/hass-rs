/// Classification of a non-primary entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "ser", derive(serde::Serialize))]
#[cfg_attr(feature = "de", derive(serde::Deserialize))]
pub enum EntityCategory {
	/// Default - uncategorized entity.
	None,

	/// An entity which allows changing the configuration of a device,
	/// for example a switch entity making it possible to turn the
	/// background illumination of a switch on and off.
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "config"))]
	Config,

	/// An entity exposing some configuration parameter or diagnostics
	/// of a device but does not allow changing it, for example a sensor
	/// showing RSSI or MAC-address.
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "diagnostic"))]
	Diagnostic,

	/// An entity which is not useful for the user to interact with.
	/// As an example the auto generated energy cost sensors are not useful
	/// on their own because they reset from 0 every time home assistant is
	/// restarted or the energy settings are changed and thus have their
	/// entity category set to [System][EntityCategory::System].
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "system"))]
	System,
}

impl EntityCategory {
	#[inline]
	pub const fn is_none(&self) -> bool {
		matches!(self, Self::None)
	}
}

impl Default for EntityCategory {
	#[inline]
	fn default() -> Self {
		Self::None
	}
}
