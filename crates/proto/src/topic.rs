use semval::{context::Context, Validate};

pub use crate::string_wrappers::Topic;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TopicInvalidity {
  Empty,
  IllegalCharacter,
}

impl<'a> Validate for Topic<'a> {
  type Invalidity = TopicInvalidity;

  fn validate(&self) -> semval::Result<Self::Invalidity> {
    Context::new()
      .invalidate_if(self.is_empty(), TopicInvalidity::Empty)
      .invalidate_if(
        self.contains(|c| matches!(c, '#' | '+')),
        TopicInvalidity::IllegalCharacter,
      )
      .into()
  }
}
