use semval::{context::Context, Validate, ValidationResult};

pub use crate::string::Topic;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TopicInvalidity {
	Empty,
	IllegalCharacter,
}

impl<'a> Validate for Topic<'a> {
	type Invalidity = TopicInvalidity;

	fn validate(&self) -> ValidationResult<Self::Invalidity> {
		Context::new()
			.invalidate_if(self.is_empty(), TopicInvalidity::Empty)
			.invalidate_if(
				self.contains(|c| matches!(c, '#' | '+')),
				TopicInvalidity::IllegalCharacter,
			)
			.into()
	}
}

#[cfg(feature = "alloc")]
#[cfg(test)]
mod tests {
	use super::*;
	use alloc::vec::Vec;

	#[test]
	fn empty_topic_is_invalid() {
		let err: Vec<_> = Topic::from("")
			.validate()
			.expect_err("should be invalid")
			.into_iter()
			.collect();

		assert_eq!(&*err, &[TopicInvalidity::Empty])
	}

	#[test]
	fn pound_symbol_in_topic_is_invalid() {
		let err: Vec<_> = Topic::from("foo/#/bar")
			.validate()
			.expect_err("should be invalid")
			.into_iter()
			.collect();

		assert_eq!(&*err, &[TopicInvalidity::IllegalCharacter])
	}

	#[test]
	fn plus_symbol_in_topic_is_invalid() {
		let err: Vec<_> = Topic::from("foo/+/bar")
			.validate()
			.expect_err("should be invalid")
			.into_iter()
			.collect();

		assert_eq!(&*err, &[TopicInvalidity::IllegalCharacter])
	}
}
