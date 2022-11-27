use crate::{AttributeKey, AttributeValue, Attributes, EntityState, EntityStateValue};

pub struct Sensor<'a, S, K, V>
where
	K: AttributeKey<'a>,
	V: AttributeValue<'a>,
	S: EntityStateValue,
{
	state: S,
	attributes: Attributes<'a, K, V, Self>,
}

impl<'a, S, K, V> EntityState<'a, K, V> for Sensor<'a, S, K, V>
where
	K: AttributeKey<'a>,
	V: AttributeValue<'a>,
	S: EntityStateValue,
{
	type State = S;

	fn get(&self) -> &Self::State {
		&self.state
	}

	fn get_mut(&mut self) -> &mut Self::State {
		&mut self.state
	}

	fn attributes(&self) -> &Attributes<'a, K, V, Self> {
		&self.attributes
	}

	fn attributes_mut(&mut self) -> &mut Attributes<'a, K, V, Self> {
		&mut self.attributes
	}
}
