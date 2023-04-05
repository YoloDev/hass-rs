use semval::{context::Context, Validate, ValidationResult};

pub use crate::string::Template;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TemplateInvalidity {
	Empty,
}

impl<'a> Validate for Template<'a> {
	type Invalidity = TemplateInvalidity;

	fn validate(&self) -> ValidationResult<Self::Invalidity> {
		Context::new()
			.invalidate_if(self.is_empty(), TemplateInvalidity::Empty)
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
		let err: Vec<_> = Template::from("")
			.validate()
			.expect_err("should be invalid")
			.into_iter()
			.collect();

		assert_eq!(&*err, &[TemplateInvalidity::Empty])
	}
}
