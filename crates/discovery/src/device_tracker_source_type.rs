use serde::{Deserialize, Serialize};

/// Attribute of a device tracker that affects state when being used to track a person.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DeviceTrackerSourceType {
	#[serde(rename = "gps")]
	GPS,

	#[serde(rename = "router")]
	Router,

	#[serde(rename = "bluetooth")]
	Bluetooth,

	/// Bluetooth Low Energy
	#[serde(rename = "bluetooth_le")]
	BluetoothLE,
}
