use semval::{context::Context, Invalidity, Validate};

pub trait ValidateContextExt {
  type Invalidity: Invalidity;

  /// Validate the target and merge the mapped result into this context if the target is not `None`.
  fn validate_with_opt<F, U>(self, target: &Option<impl Validate<Invalidity = U>>, map: F) -> Self
  where
    F: Fn(U) -> Self::Invalidity,
    U: Invalidity;

  /// Validate all items in an iterator.
  fn validate_iter<'a, F, U, I, II: 'a>(self, target: I, map: F) -> Self
  where
    F: Fn(usize, U) -> Self::Invalidity,
    U: Invalidity,
    I: IntoIterator<Item = &'a II>,
    II: Validate<Invalidity = U>;
  // /// Validate the target and merge the mapped result into this context
  // #[inline]
  // pub fn validate_with<F, U>(self, target: &impl Validate<Invalidity = U>, map: F) -> Self
  // where
  //     F: Fn(U) -> V,
  //     U: Invalidity,
  // {
  //     self.merge_result_with(target.validate(), map)
  // }
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

  fn validate_iter<'a, F, U, I, II: 'a>(self, target: I, map: F) -> Self
  where
    F: Fn(usize, U) -> Self::Invalidity,
    U: Invalidity,
    I: IntoIterator<Item = &'a II>,
    II: Validate<Invalidity = U>,
  {
    let mut ret = self;

    for (index, item) in target.into_iter().enumerate() {
      ret = ret.validate_with(item, |v| map(index, v));
    }

    ret
  }
}
