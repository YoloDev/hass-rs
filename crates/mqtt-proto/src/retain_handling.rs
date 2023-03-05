use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(
	Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize_repr, Deserialize_repr,
)]
#[repr(u8)]
pub enum MqttRetainHandling {
	/// Send retained messages at the time of the subscribe
	SendRetainedOnSubscribe = 0,
	/// Send retained messages on subscribe only if subscription is new
	SendRetainedOnNew = 1,
	/// Do not send retained messages at all
	DontSendRetained = 2,
}

impl Default for MqttRetainHandling {
	fn default() -> Self {
		Self::SendRetainedOnSubscribe
	}
}

impl MqttRetainHandling {
	#[inline]
	pub const fn is_default(&self) -> bool {
		matches!(self, MqttRetainHandling::SendRetainedOnSubscribe)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_test::{assert_tokens, Token};

	#[test]
	fn serde_as_number() {
		assert_tokens(
			&MqttRetainHandling::SendRetainedOnSubscribe,
			&[Token::U8(0)],
		)
	}
}
