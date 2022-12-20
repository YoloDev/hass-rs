mod serde;

use crate::{
	name::{Name, NameInvalidity},
	validation::ValidateContextExt,
};
use ::serde::{Deserialize, Serialize};
use semval::{context::Context, Validate, ValidationResult};
use std::borrow::Cow;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device<'a> {
	/// A list of connections of the device to the outside world as a list of tuples [connection_type, connection_identifier].
	/// For example the MAC address of a network interface: "connections": [["mac", "02:5b:26:a8:dc:12"]].
	#[serde(borrow, default, skip_serializing_if = "<[ConnectionInfo]>::is_empty")]
	pub connections: Cow<'a, [ConnectionInfo<'a>]>,

	/// A list of IDs that uniquely identify the device. For example a serial number.
	#[serde(borrow, default, skip_serializing_if = "<[Cow<str>]>::is_empty")]
	pub identifiers: Cow<'a, [Cow<'a, str>]>,

	/// The manufacturer of the device.
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub manufacturer: Option<Cow<'a, str>>,

	/// The model of the device.
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub model: Option<Cow<'a, str>>,

	/// The name of the device.
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub name: Option<Name<'a>>,

	/// Suggest an area if the device isn’t in one yet.
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub suggested_area: Option<Cow<'a, str>>,

	/// The firmware version of the device.
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub sw_version: Option<Cow<'a, str>>,

	/// The hardware version of the device.
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub hw_version: Option<Cow<'a, str>>,

	/// Identifier of a device that routes messages between this device and Home Assistant.
	/// Examples of such devices are hubs, or parent devices of a sub-device. This is used
	/// to show device topology in Home Assistant.
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub via_device: Option<Cow<'a, str>>,

	/// A link to the webpage that can manage the configuration of this device.
	/// Can be either an HTTP or HTTPS link.
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub configuration_url: Option<Cow<'a, str>>,
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
	pub type_name: Cow<'a, str>,

	/// Connection value. For instance `02:5b:26:a8:dc:12` for a mac-address.
	pub value: Cow<'a, str>,
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
				type_name: Cow::Borrowed("ty"),
				value: Cow::Borrowed("val"),
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
		assert_matches!(connection_info.type_name, Cow::Borrowed(_));
		assert_matches!(connection_info.value, Cow::Borrowed(_));
	}

	#[test]
	fn empty_device_serde() {
		assert_tokens(
			&Device {
				connections: Cow::Borrowed(&[]),
				identifiers: Cow::Borrowed(&[]),
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
				connections: Cow::Borrowed(&[
					ConnectionInfo {
						type_name: Cow::Borrowed("type1"),
						value: Cow::Borrowed("val1"),
					},
					ConnectionInfo {
						type_name: Cow::Borrowed("type2"),
						value: Cow::Borrowed("val2"),
					},
				]),
				identifiers: Cow::Borrowed(&[Cow::Borrowed("id1"), Cow::Borrowed("id2")]),
				manufacturer: Some(Cow::Borrowed("mf")),
				model: Some(Cow::Borrowed("md")),
				name: Some(Name(Cow::Borrowed("na"))),
				suggested_area: Some(Cow::Borrowed("ar")),
				sw_version: Some(Cow::Borrowed("sw")),
				hw_version: Some(Cow::Borrowed("hw")),
				via_device: Some(Cow::Borrowed("vd")),
				configuration_url: Some(Cow::Borrowed("cu")),
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
