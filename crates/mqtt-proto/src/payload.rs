use semval::{context::Context, Validate, ValidationResult};

pub use crate::string::Payload;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PayloadInvalidity {
	Empty,
}

impl<'a> Validate for Payload<'a> {
	type Invalidity = PayloadInvalidity;

	fn validate(&self) -> ValidationResult<Self::Invalidity> {
		Context::new()
			.invalidate_if(self.is_empty(), PayloadInvalidity::Empty)
			.into()
	}
}

#[cfg(feature = "alloc")]
#[cfg(test)]
mod tests {
	use super::*;
	use alloc::vec::Vec;

	#[test]
	fn empty_payload_is_invalid() {
		let err: Vec<_> = Payload::from("")
			.validate()
			.expect_err("should be invalid")
			.into_iter()
			.collect();

		assert_eq!(&*err, &[PayloadInvalidity::Empty])
	}
}
