#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "ser", derive(serde_repr::Serialize_repr))]
#[cfg_attr(feature = "de", derive(serde_repr::Deserialize_repr))]
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
#[cfg(all(feature = "ser", feature = "de"))]
mod tests {
	use super::*;
	use serde_test::{assert_tokens, Token};

	#[test]
	fn serde_as_number() {
		assert_tokens(&MqttQoS::AtMostOnce, &[Token::U8(0)])
	}
}
