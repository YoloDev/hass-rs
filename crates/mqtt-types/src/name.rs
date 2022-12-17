use semval::{context::Context, Validate, ValidationResult};

pub use crate::string_wrappers::Name;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NameInvalidity {
	Empty,
}

impl<'a> Validate for Name<'a> {
	type Invalidity = NameInvalidity;

	fn validate(&self) -> ValidationResult<Self::Invalidity> {
		Context::new()
			.invalidate_if(self.is_empty(), NameInvalidity::Empty)
			.into()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn empty_payload_is_invalid() {
		let err: Vec<_> = Name::from("")
			.validate()
			.expect_err("should be invalid")
			.into_iter()
			.collect();

		assert_eq!(&*err, &[NameInvalidity::Empty])
	}
}
