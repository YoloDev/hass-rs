mod attributes;
mod sensor;

pub use attributes::{AttributeKey, AttributeValue, Attributes};

pub trait EntityState<'a, K, V>
where
  K: AttributeKey<'a>,
  V: AttributeValue<'a>,
{
  type State;

  fn get(&self) -> &Self::State;
  fn get_mut(&mut self) -> &mut Self::State;

  fn attributes(&self) -> &Attributes<'a, K, V, Self>;
  fn attributes_mut(&mut self) -> &mut Attributes<'a, K, V, Self>;
}
