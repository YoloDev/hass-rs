mod serde;

use crate::EntityState;
use std::{
	borrow::{Borrow, Cow},
	collections::{btree_map, BTreeMap},
	fmt,
	marker::PhantomData,
};

pub trait AttributeValue<'a>: Eq + Borrow<str> + ::serde::Deserialize<'a> {
	fn from_cow(value: Cow<'a, str>) -> Self;
}
pub trait AttributeKey<'a>: Ord + AttributeValue<'a> {}

impl<'a> AttributeValue<'a> for String {
	fn from_cow(value: Cow<'a, str>) -> Self {
		value.into_owned()
	}
}

impl<'a> AttributeValue<'a> for Cow<'a, str> {
	fn from_cow(value: Cow<'a, str>) -> Self {
		value
	}
}

impl<'a> AttributeKey<'a> for String {}
impl<'a> AttributeKey<'a> for Cow<'a, str> {}

pub struct Attributes<'a, K, V, E: ?Sized>(BTreeMap<K, V>, PhantomData<fn() -> &'a E>)
where
	K: AttributeKey<'a>,
	V: AttributeValue<'a>,
	E: EntityState<'a, K, V>;

impl<'a, K, V, E> Default for Attributes<'a, K, V, E>
where
	K: AttributeKey<'a>,
	V: AttributeValue<'a>,
	E: EntityState<'a, K, V>,
{
	fn default() -> Self {
		Self(BTreeMap::default(), PhantomData)
	}
}

impl<'a, K, V, E> fmt::Debug for Attributes<'a, K, V, E>
where
	K: AttributeKey<'a>,
	V: AttributeValue<'a>,
	E: EntityState<'a, K, V>,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_map()
			.entries(self.0.iter().map(|(k, v)| (k.borrow(), v.borrow())))
			.finish()
	}
}

impl<'a, K, V, E> PartialEq for Attributes<'a, K, V, E>
where
	K: AttributeKey<'a>,
	V: AttributeValue<'a>,
	E: EntityState<'a, K, V>,
{
	fn eq(&self, other: &Self) -> bool {
		self.0.eq(&other.0)
	}
}

impl<'a, K, V, E> Attributes<'a, K, V, E>
where
	K: AttributeKey<'a>,
	V: AttributeValue<'a>,
	E: EntityState<'a, K, V>,
{
	pub fn get(&self, key: &str) -> Option<&str> {
		self.get_inner(key).map(|v| (*v).borrow())
	}

	pub fn get_inner(&self, key: &str) -> Option<&V> {
		self.0.get(key)
	}

	pub fn insert(&mut self, key: K, value: V) -> Option<V> {
		self.0.insert(key, value)
	}

	pub fn remove(&mut self, key: &str) -> Option<V> {
		self.0.remove(key)
	}
}

pub struct Entry<'a, 'b, K, V>
where
	K: AttributeKey<'a>,
	V: AttributeValue<'a>,
{
	inner: btree_map::Entry<'b, K, V>,
	marker: PhantomData<&'a ()>,
}

impl<'a, 'b, K, V> Entry<'a, 'b, K, V>
where
	K: AttributeKey<'a>,
	V: AttributeValue<'a>,
{
	pub fn key(&self) -> &str {
		self.key_inner().borrow()
	}

	pub fn key_inner(&self) -> &K {
		self.inner.key()
	}

	pub fn or_insert(self, default: V) -> &'b mut V {
		self.inner.or_insert(default)
	}

	pub fn or_insert_with(self, default: impl FnOnce() -> V) -> &'b mut V {
		self.inner.or_insert_with(default)
	}

	pub fn and_modify(self, f: impl FnOnce(&mut V)) -> Self {
		Entry {
			inner: self.inner.and_modify(f),
			marker: PhantomData,
		}
	}
}

impl<'a, K, V, E> Attributes<'a, K, V, E>
where
	K: AttributeKey<'a>,
	V: AttributeValue<'a>,
	E: EntityState<'a, K, V>,
{
	pub fn entry<'b>(&'b mut self, key: K) -> Entry<'a, 'b, K, V> {
		Entry {
			inner: self.0.entry(key),
			marker: PhantomData,
		}
	}
}
