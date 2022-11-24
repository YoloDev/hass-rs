use error_stack::Context;
use semval::Invalidity;
use std::fmt;

#[derive(Debug, Clone)]
pub struct ValidationError<I: Invalidity + Send + Sync>(I);

impl<I: Invalidity + Send + Sync> fmt::Display for ValidationError<I> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "validation error: {:?}", self.0)
  }
}

impl<I: Invalidity + Send + Sync> Context for ValidationError<I> {}

impl<I: Invalidity + Send + Sync> ValidationError<I> {
  pub fn new(invalidity: I) -> Self {
    Self(invalidity)
  }
}

pub(crate) trait CustomValidation {
  type Invalidity: Invalidity;

  fn additional_validation(
    &self,
    context: semval::context::Context<Self::Invalidity>,
  ) -> semval::context::Context<Self::Invalidity>;
}

pub(crate) trait CustomValidationExt {
  type Invalidity: Invalidity;

  fn validate_entity(
    self,
    custom_validatable: &impl CustomValidation<Invalidity = Self::Invalidity>,
  ) -> Self;
}

impl<I: Invalidity> CustomValidationExt for semval::context::Context<I> {
  type Invalidity = I;

  #[inline]
  fn validate_entity(
    self,
    custom_validatable: &impl CustomValidation<Invalidity = Self::Invalidity>,
  ) -> Self {
    custom_validatable.additional_validation(self)
  }
}
