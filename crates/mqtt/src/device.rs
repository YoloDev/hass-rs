use hass_mqtt_client::proto::device::ConnectionInfo;
use std::borrow::Cow;

// pub trait Device: Send + Sync {
// 	/// A list of IDs that uniquely identify the device. For example a serial number.
// 	fn identifiers(&self) -> &Identifiers;

// 	/// A list of connections of the device to the outside world as a list of tuples [connection_type, connection_identifier].
// 	/// For example the MAC address of a network interface: "connections": [["mac", "02:5b:26:a8:dc:12"]].
// 	fn connections(&self) -> Option<&Connections> {
// 		None
// 	}

// 	/// The manufacturer of the device.
// 	fn manufacturer(&self) -> Option<&str> {
// 		None
// 	}

// 	/// The model of the device.
// 	fn model(&self) -> Option<&str> {
// 		None
// 	}

// 	/// The name of the device.
// 	fn name(&self) -> Option<&str> {
// 		None
// 	}

// 	/// Suggest an area if the device isn’t in one yet.
// 	fn suggested_area(&self) -> Option<&str> {
// 		None
// 	}

// 	/// The firmware version of the device.
// 	fn sw_version(&self) -> Option<&str> {
// 		None
// 	}

// 	/// The hardware version of the device.
// 	fn hw_version(&self) -> Option<&str> {
// 		None
// 	}

// 	/// Identifier of a device that routes messages between this device and Home Assistant.
// 	/// Examples of such devices are hubs, or parent devices of a sub-device. This is used
// 	/// to show device topology in Home Assistant.
// 	fn via_device(&self) -> Option<&str> {
// 		None
// 	}

// 	/// A link to the webpage that can manage the configuration of this device.
// 	/// Can be either an HTTP or HTTPS link.
// 	fn configuration_url(&self) -> Option<&str> {
// 		None
// 	}
// }

// static_assertions::assert_obj_safe!(Device);

// pub struct Identifiers {
// 	identifiers: Vec<Cow<'static, str>>,
// }

// pub struct Connections {
// 	connections: Vec<ConnectionInfo<'static>>,
// }
