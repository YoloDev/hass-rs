use semval::{context::Context, Validate};

pub use crate::string_wrappers::Icon;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum IconInvalidity {
  Empty,
}

impl<'a> Validate for Icon<'a> {
  type Invalidity = IconInvalidity;

  fn validate(&self) -> semval::Result<Self::Invalidity> {
    Context::new()
      .invalidate_if(self.is_empty(), IconInvalidity::Empty)
      .into()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn empty_payload_is_invalid() {
    let err: Vec<_> = Icon::from("")
      .validate()
      .expect_err("should be invalid")
      .into_iter()
      .collect();

    assert_eq!(&*err, &[IconInvalidity::Empty])
  }
}
