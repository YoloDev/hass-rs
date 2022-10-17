use serde::{Deserialize, Serialize};

/// Attribute of a device tracker that affects state when being used to track a person.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SourceType {
  #[serde(rename = "gps")]
  GPS,

  #[serde(rename = "router")]
  Router,

  #[serde(rename = "bluetooth")]
  Bluetooth,

  #[serde(rename = "bluetooth_le")]
  BluetoothLE,
}
