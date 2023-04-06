use super::ConnectionInfo;
use crate::HassStr;

#[cfg_attr(feature = "ser", derive(::serde::Serialize))]
#[cfg_attr(feature = "de", derive(::serde::Deserialize))]
#[serde(rename = "ConnectionInfo")]
struct ConnectionInfoTuple<'a>(#[serde(borrow)] HassStr<'a>, #[serde(borrow)] HassStr<'a>);

#[cfg(feature = "ser")]
impl<'a> ::serde::Serialize for ConnectionInfo<'a> {
	#[inline]
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		use nameof::name_of_type;
		use serde::ser::SerializeTupleStruct;

		let mut serializer = serializer.serialize_tuple_struct(name_of_type!(ConnectionInfo), 2)?;
		serializer.serialize_field(&self.type_name)?;
		serializer.serialize_field(&self.value)?;
		serializer.end()
	}
}

#[cfg(feature = "de")]
impl<'a, 'de: 'a> ::serde::Deserialize<'de> for ConnectionInfo<'a> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let ConnectionInfoTuple(type_name, value) = ::serde::Deserialize::deserialize(deserializer)?;
		Ok(ConnectionInfo { type_name, value })
	}
}
