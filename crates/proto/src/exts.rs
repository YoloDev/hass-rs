use semval::{context::Context, Invalidity, Validate};

pub trait ValidateContextExt {
  type Invalidity: Invalidity;

  /// Validate the target and merge the mapped result into this context if the target is not `None`.
  fn validate_with_opt<F, U>(self, target: &Option<impl Validate<Invalidity = U>>, map: F) -> Self
  where
    F: Fn(U) -> Self::Invalidity,
    U: Invalidity;
}

impl<V: Invalidity> ValidateContextExt for Context<V> {
  type Invalidity = V;

  #[inline]
  fn validate_with_opt<F, U>(self, target: &Option<impl Validate<Invalidity = U>>, map: F) -> Self
  where
    F: Fn(U) -> Self::Invalidity,
    U: Invalidity,
  {
    match target {
      Some(v) => self.validate_with(v, map),
      None => self,
    }
  }
}
