/// Attribute of a device tracker that affects state when being used to track a person.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "ser", derive(serde::Serialize))]
#[cfg_attr(feature = "de", derive(serde::Deserialize))]
pub enum DeviceTrackerSourceType {
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "gps"))]
	GPS,

	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "router"))]
	Router,

	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "bluetooth"))]
	Bluetooth,

	/// Bluetooth Low Energy
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "bluetooth_le"))]
	BluetoothLE,
}
