use core::hash::Hash;

#[cfg(feature = "alloc")]
use alloc::{sync::Arc, vec::Vec};

pub enum HassItems<'a, T> {
	Borrowed(&'a [T]),
	#[cfg(feature = "alloc")]
	Vec(Vec<T>),
	#[cfg(feature = "alloc")]
	Arc(Arc<[T]>),
}

impl<'a, T> HassItems<'a, T> {
	#[inline]
	pub const fn is_borrowed(&self) -> bool {
		matches!(self, Self::Borrowed(_))
	}

	#[inline]
	#[cfg(feature = "alloc")]
	pub const fn is_vec(&self) -> bool {
		matches!(self, Self::Vec(_))
	}

	#[inline]
	#[cfg(feature = "alloc")]
	pub const fn is_arc(&self) -> bool {
		matches!(self, Self::Arc(_))
	}

	#[inline]
	pub fn as_slice(&self) -> &[T] {
		match self {
			Self::Borrowed(s) => s,
			#[cfg(feature = "alloc")]
			Self::Vec(s) => s.as_ref(),
			#[cfg(feature = "alloc")]
			Self::Arc(s) => s.as_ref(),
		}
	}

	#[inline]
	#[cfg(feature = "alloc")]
	pub fn into_arc(self) -> Arc<[T]>
	where
		T: Clone,
	{
		match self {
			Self::Borrowed(s) => Arc::from_iter(s.iter().cloned()),
			Self::Vec(s) => Arc::from(s),
			Self::Arc(s) => s,
		}
	}

	#[inline]
	#[cfg(feature = "alloc")]
	pub fn as_arc(&mut self) -> &Arc<[T]>
	where
		T: Clone,
	{
		if let Self::Arc(s) = self {
			return s;
		}

		let old_value = core::mem::replace(self, Self::Borrowed(&[]));
		*self = Self::Arc(match old_value {
			Self::Borrowed(s) => Arc::from_iter(s.iter().cloned()),
			Self::Vec(s) => Arc::from(s),
			Self::Arc(_) => unreachable!(),
		});

		let Self::Arc(s) = self else { unreachable!() };
		s
	}

	pub fn iter(&self) -> core::slice::Iter<T> {
		self.as_slice().iter()
	}
}

impl<'a, T> Default for HassItems<'a, T> {
	fn default() -> Self {
		Self::Borrowed(&[])
	}
}

impl<'a, T> IntoIterator for &'a HassItems<'_, T> {
	type Item = &'a T;
	type IntoIter = core::slice::Iter<'a, T>;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

impl<'a, T: Clone> Clone for HassItems<'a, T> {
	fn clone(&self) -> Self {
		match self {
			Self::Borrowed(s) => Self::Borrowed(s),
			#[cfg(feature = "alloc")]
			Self::Vec(v) => Self::Arc(Arc::from_iter(v.iter().cloned())),
			#[cfg(feature = "alloc")]
			Self::Arc(s) => Self::Arc(s.clone()),
		}
	}
}

impl<'a, T: Eq> Eq for HassItems<'a, T> {}
impl<'a, T: Ord> Ord for HassItems<'a, T> {
	fn cmp(&self, other: &Self) -> core::cmp::Ordering {
		self.as_ref().cmp(other.as_ref())
	}
}

impl<'a, T: PartialEq> PartialEq for HassItems<'a, T> {
	fn eq(&self, other: &Self) -> bool {
		self.as_ref() == other.as_ref()
	}
}

impl<'a, T: PartialOrd> PartialOrd for HassItems<'a, T> {
	fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
		self.as_ref().partial_cmp(other.as_ref())
	}
}

impl<'a, T: Hash> Hash for HassItems<'a, T> {
	fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
		self.as_ref().hash(state)
	}
}

impl<'a, T> From<&'a [T]> for HassItems<'a, T> {
	#[inline]
	fn from(value: &'a [T]) -> Self {
		Self::Borrowed(value)
	}
}

#[cfg(feature = "alloc")]
impl<'a, T> From<Arc<[T]>> for HassItems<'a, T> {
	#[inline]
	fn from(value: Arc<[T]>) -> Self {
		Self::Arc(value)
	}
}

#[cfg(feature = "alloc")]
impl<'a, T> From<Vec<T>> for HassItems<'a, T> {
	#[inline]
	fn from(value: Vec<T>) -> Self {
		Self::Vec(value)
	}
}

impl<'a, T> AsRef<[T]> for HassItems<'a, T> {
	fn as_ref(&self) -> &[T] {
		match self {
			Self::Borrowed(s) => s,
			#[cfg(feature = "alloc")]
			Self::Vec(s) => s,
			#[cfg(feature = "alloc")]
			Self::Arc(s) => s,
		}
	}
}

impl<'a, T: core::fmt::Debug> core::fmt::Debug for HassItems<'a, T> {
	#[inline]
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		core::fmt::Debug::fmt(&**self, f)
	}
}

impl<'a, T> core::ops::Deref for HassItems<'a, T> {
	type Target = [T];

	#[inline]
	fn deref(&self) -> &Self::Target {
		self.as_ref()
	}
}

#[cfg(feature = "alloc")]
impl<'a, T> FromIterator<T> for HassItems<'a, T> {
	fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
		Self::Arc(Arc::from_iter(iter.into_iter()))
	}
}

#[cfg(feature = "ser")]
impl<'a, T: serde::Serialize> serde::Serialize for HassItems<'a, T> {
	#[inline]
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		match self {
			Self::Borrowed(s) => s.serialize(serializer),
			#[cfg(feature = "alloc")]
			Self::Vec(s) => s.serialize(serializer),
			#[cfg(feature = "alloc")]
			Self::Arc(s) => s.serialize(serializer),
		}
	}
}

#[cfg(all(feature = "de", feature = "alloc"))]
impl<'a, 'de: 'a, T: serde::Deserialize<'de>> serde::Deserialize<'de> for HassItems<'a, T> {
	#[inline]
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		<Vec<T> as serde::Deserialize>::deserialize(deserializer).map(|vec| Self::Arc(Arc::from(vec)))
	}
}
