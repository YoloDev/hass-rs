use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(
	Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize_repr, Deserialize_repr,
)]
#[repr(u8)]
pub enum MqttQoS {
	AtMostOnce = 0,
	AtLeastOnce = 1,
	ExactlyOnce = 2,
}

impl Default for MqttQoS {
	fn default() -> Self {
		Self::AtMostOnce
	}
}

impl MqttQoS {
	#[inline]
	pub const fn is_default(&self) -> bool {
		matches!(self, MqttQoS::AtMostOnce)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_test::{assert_tokens, Token};

	#[test]
	fn serde_as_number() {
		assert_tokens(&MqttQoS::AtMostOnce, &[Token::U8(0)])
	}
}
