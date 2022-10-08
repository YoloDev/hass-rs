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
