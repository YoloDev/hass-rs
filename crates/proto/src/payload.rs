use semval::{context::Context, Validate};

pub use crate::string_wrappers::Payload;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PayloadInvalidity {
  Empty,
}

impl<'a> Validate for Payload<'a> {
  type Invalidity = PayloadInvalidity;

  fn validate(&self) -> semval::Result<Self::Invalidity> {
    Context::new()
      .invalidate_if(self.is_empty(), PayloadInvalidity::Empty)
      .into()
  }
}
