use nameof::name_of_type;
use serde::{ser::SerializeTupleStruct, Deserialize, Serialize};
use std::borrow::Cow;

use super::ConnectionInfo;

#[derive(Serialize, Deserialize)]
#[serde(rename = "ConnectionInfo")]
struct ConnectionInfoTuple<'a>(#[serde(borrow)] Cow<'a, str>, #[serde(borrow)] Cow<'a, str>);

impl<'a> Serialize for ConnectionInfo<'a> {
	#[inline]
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let mut serializer = serializer.serialize_tuple_struct(name_of_type!(ConnectionInfo), 2)?;
		serializer.serialize_field(&self.type_name)?;
		serializer.serialize_field(&self.value)?;
		serializer.end()
	}
}

impl<'a, 'de: 'a> Deserialize<'de> for ConnectionInfo<'a> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let ConnectionInfoTuple(type_name, value) = Deserialize::deserialize(deserializer)?;
		Ok(ConnectionInfo { type_name, value })
	}
}
