use semval::{context::Context, Validate};

pub use crate::string_wrappers::UniqueId;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum UniqueIdInvalidity {
  Empty,
}

impl<'a> Validate for UniqueId<'a> {
  type Invalidity = UniqueIdInvalidity;

  fn validate(&self) -> semval::Result<Self::Invalidity> {
    Context::new()
      .invalidate_if(self.is_empty(), UniqueIdInvalidity::Empty)
      .into()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn empty_payload_is_invalid() {
    let err: Vec<_> = UniqueId::from("")
      .validate()
      .expect_err("should be invalid")
      .into_iter()
      .collect();

    assert_eq!(&*err, &[UniqueIdInvalidity::Empty])
  }
}
