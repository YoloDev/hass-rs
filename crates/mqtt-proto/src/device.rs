#[cfg(any(feature = "ser", feature = "de"))]
mod serde;

use crate::{
	name::{Name, NameInvalidity},
	validation::ValidateContextExt,
	HassItems, HassStr,
};
use semval::{context::Context, Validate, ValidationResult};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ser", derive(::serde::Serialize))]
#[cfg_attr(feature = "de", derive(::serde::Deserialize))]
pub struct Device<'a> {
	/// A list of connections of the device to the outside world as a list of tuples [connection_type, connection_identifier].
	/// For example the MAC address of a network interface: "connections": [["mac", "02:5b:26:a8:dc:12"]].
	#[cfg_attr(
		any(feature = "ser", feature = "de"),
		serde(borrow, default, skip_serializing_if = "<[_]>::is_empty")
	)]
	pub connections: HassItems<'a, ConnectionInfo<'a>>,

	/// A list of IDs that uniquely identify the device. For example a serial number.
	#[cfg_attr(
		any(feature = "ser", feature = "de"),
		serde(borrow, default, skip_serializing_if = "<[_]>::is_empty")
	)]
	pub identifiers: HassItems<'a, HassStr<'a>>,

	/// The manufacturer of the device.
	#[cfg_attr(
		any(feature = "ser", feature = "de"),
		serde(borrow, default, skip_serializing_if = "Option::is_none")
	)]
	pub manufacturer: Option<HassStr<'a>>,

	/// The model of the device.
	#[cfg_attr(
		any(feature = "ser", feature = "de"),
		serde(borrow, default, skip_serializing_if = "Option::is_none")
	)]
	pub model: Option<HassStr<'a>>,

	#[cfg_attr(
		any(feature = "ser", feature = "de"),
		serde(borrow, default, skip_serializing_if = "Option::is_none")
	)]
	pub name: Option<Name<'a>>,

	/// Suggest an area if the device isn’t in one yet.
	#[cfg_attr(
		any(feature = "ser", feature = "de"),
		serde(borrow, default, skip_serializing_if = "Option::is_none")
	)]
	pub suggested_area: Option<HassStr<'a>>,

	/// The firmware version of the device.
	#[cfg_attr(
		any(feature = "ser", feature = "de"),
		serde(borrow, default, skip_serializing_if = "Option::is_none")
	)]
	pub sw_version: Option<HassStr<'a>>,

	/// The hardware version of the device.
	#[cfg_attr(
		any(feature = "ser", feature = "de"),
		serde(borrow, default, skip_serializing_if = "Option::is_none")
	)]
	pub hw_version: Option<HassStr<'a>>,

	/// Identifier of a device that routes messages between this device and Home Assistant.
	/// Examples of such devices are hubs, or parent devices of a sub-device. This is used
	/// to show device topology in Home Assistant.
	#[cfg_attr(
		any(feature = "ser", feature = "de"),
		serde(borrow, default, skip_serializing_if = "Option::is_none")
	)]
	pub via_device: Option<HassStr<'a>>,

	/// A link to the webpage that can manage the configuration of this device.
	/// Can be either an HTTP or HTTPS link.
	#[cfg_attr(
		any(feature = "ser", feature = "de"),
		serde(borrow, default, skip_serializing_if = "Option::is_none")
	)]
	pub configuration_url: Option<HassStr<'a>>,
}

impl<'a> Device<'a> {
	pub fn is_empty(&self) -> bool {
		self.connections.is_empty()
			&& self.identifiers.is_empty()
			&& self.manufacturer.is_none()
			&& self.model.is_none()
			&& self.name.is_none()
			&& self.suggested_area.is_none()
			&& self.sw_version.is_none()
			&& self.hw_version.is_none()
			&& self.via_device.is_none()
			&& self.configuration_url.is_none()
	}
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DeviceInvalidity {
	Connection(usize, ConnectionInfoInvalidity),
	Name(NameInvalidity),
}

impl<'a> Validate for Device<'a> {
	type Invalidity = DeviceInvalidity;

	fn validate(&self) -> ValidationResult<Self::Invalidity> {
		Context::new()
			.validate_iter(&*self.connections, DeviceInvalidity::Connection)
			.validate_with_opt(&self.name, DeviceInvalidity::Name)
			.into()
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionInfo<'a> {
	/// Connection type. For example `mac` for a mac-addresses.
	pub type_name: HassStr<'a>,

	/// Connection value. For instance `02:5b:26:a8:dc:12` for a mac-address.
	pub value: HassStr<'a>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ConnectionInfoInvalidity {
	TypeNameEmpty,
	ValueEmpty,
}

impl<'a> Validate for ConnectionInfo<'a> {
	type Invalidity = ConnectionInfoInvalidity;

	fn validate(&self) -> ValidationResult<Self::Invalidity> {
		Context::new()
			.invalidate_if(
				self.type_name.is_empty(),
				ConnectionInfoInvalidity::TypeNameEmpty,
			)
			.invalidate_if(self.value.is_empty(), ConnectionInfoInvalidity::ValueEmpty)
			.into()
	}
}

#[cfg(all(feature = "ser", feature = "de"))]
#[cfg(test)]
mod tests {
	use super::*;
	use assert_matches::assert_matches;
	use nameof::{name_of, name_of_type};
	use serde_test::{assert_tokens, Token};

	#[test]
	fn connection_info_serde() {
		assert_tokens(
			&ConnectionInfo {
				type_name: HassStr::Borrowed("ty"),
				value: HassStr::Borrowed("val"),
			},
			&[
				Token::TupleStruct {
					name: name_of_type!(ConnectionInfo),
					len: 2,
				},
				Token::Str("ty"),
				Token::Str("val"),
				Token::TupleStructEnd,
			],
		)
	}

	#[test]
	fn connection_info_borrows() {
		let json = r#"["ty","val"]"#;
		let connection_info: ConnectionInfo = serde_json::from_str(json).expect("should parse");
		assert_matches!(connection_info.type_name, HassStr::Borrowed(_));
		assert_matches!(connection_info.value, HassStr::Borrowed(_));
	}

	#[test]
	fn empty_device_serde() {
		assert_tokens(
			&Device {
				connections: HassItems::Borrowed(&[]),
				identifiers: HassItems::Borrowed(&[]),
				manufacturer: None,
				model: None,
				name: None,
				suggested_area: None,
				sw_version: None,
				hw_version: None,
				via_device: None,
				configuration_url: None,
			},
			&[
				Token::Struct {
					name: name_of_type!(Device),
					len: 0,
				},
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn device_serde() {
		assert_tokens(
			&Device {
				connections: HassItems::Borrowed(&[
					ConnectionInfo {
						type_name: HassStr::Borrowed("type1"),
						value: HassStr::Borrowed("val1"),
					},
					ConnectionInfo {
						type_name: HassStr::Borrowed("type2"),
						value: HassStr::Borrowed("val2"),
					},
				]),
				identifiers: HassItems::Borrowed(&[HassStr::Borrowed("id1"), HassStr::Borrowed("id2")]),
				manufacturer: Some(HassStr::Borrowed("mf")),
				model: Some(HassStr::Borrowed("md")),
				name: Some(Name::from("na")),
				suggested_area: Some(HassStr::Borrowed("ar")),
				sw_version: Some(HassStr::Borrowed("sw")),
				hw_version: Some(HassStr::Borrowed("hw")),
				via_device: Some(HassStr::Borrowed("vd")),
				configuration_url: Some(HassStr::Borrowed("cu")),
			},
			&[
				Token::Struct {
					name: "Device",
					len: 10,
				},
				Token::Str(name_of!(connections in Device)),
				Token::Seq { len: Some(2) },
				Token::TupleStruct {
					name: name_of_type!(ConnectionInfo),
					len: 2,
				},
				Token::Str("type1"),
				Token::Str("val1"),
				Token::TupleStructEnd,
				Token::TupleStruct {
					name: name_of_type!(ConnectionInfo),
					len: 2,
				},
				Token::Str("type2"),
				Token::Str("val2"),
				Token::TupleStructEnd,
				Token::SeqEnd,
				Token::Str(name_of!(identifiers in Device)),
				Token::Seq { len: Some(2) },
				Token::Str("id1"),
				Token::Str("id2"),
				Token::SeqEnd,
				Token::Str(name_of!(manufacturer in Device)),
				Token::Some,
				Token::Str("mf"),
				Token::Str(name_of!(model in Device)),
				Token::Some,
				Token::Str("md"),
				Token::Str(name_of!(name in Device)),
				Token::Some,
				Token::Str("na"),
				Token::Str(name_of!(suggested_area in Device)),
				Token::Some,
				Token::Str("ar"),
				Token::Str(name_of!(sw_version in Device)),
				Token::Some,
				Token::Str("sw"),
				Token::Str(name_of!(hw_version in Device)),
				Token::Some,
				Token::Str("hw"),
				Token::Str(name_of!(via_device in Device)),
				Token::Some,
				Token::Str("vd"),
				Token::Str(name_of!(configuration_url in Device)),
				Token::Some,
				Token::Str("cu"),
				Token::StructEnd,
			],
		)
	}
}
