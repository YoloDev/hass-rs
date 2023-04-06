use core::hash::Hash;

#[cfg(feature = "de")]
use core::marker::PhantomData;

#[cfg(feature = "alloc")]
use alloc::{string::String, sync::Arc};

pub enum HassStr<'a> {
	Borrowed(&'a str),
	#[cfg(feature = "alloc")]
	Arc(Arc<str>),
}

impl<'a> HassStr<'a> {
	#[inline]
	pub const fn is_borrowed(&self) -> bool {
		matches!(self, Self::Borrowed(_))
	}

	#[inline]
	#[cfg(feature = "alloc")]
	pub const fn is_arc(&self) -> bool {
		matches!(self, Self::Arc(_))
	}

	#[inline]
	pub fn as_str(&self) -> &str {
		match self {
			Self::Borrowed(s) => s,
			#[cfg(feature = "alloc")]
			Self::Arc(s) => s.as_ref(),
		}
	}

	#[inline]
	#[cfg(feature = "alloc")]
	pub fn into_arc(self) -> Arc<str> {
		match self {
			Self::Borrowed(s) => Arc::from(s),
			Self::Arc(s) => s,
		}
	}

	#[inline]
	#[cfg(feature = "alloc")]
	pub fn as_arc(&mut self) -> &Arc<str> {
		match self {
			Self::Borrowed(s) => {
				*self = Self::Arc(Arc::from(*s));
				let Self::Arc(s) = self else { unreachable!() };
				s
			}
			Self::Arc(s) => s,
		}
	}
}

impl<'a> Clone for HassStr<'a> {
	fn clone(&self) -> Self {
		match self {
			Self::Borrowed(s) => Self::Borrowed(s),
			#[cfg(feature = "alloc")]
			Self::Arc(s) => Self::Arc(s.clone()),
		}
	}
}

impl<'a> Eq for HassStr<'a> {}
impl<'a> Ord for HassStr<'a> {
	fn cmp(&self, other: &Self) -> core::cmp::Ordering {
		self.as_ref().cmp(other.as_ref())
	}
}

impl<'a> PartialEq for HassStr<'a> {
	fn eq(&self, other: &Self) -> bool {
		self.as_ref() == other.as_ref()
	}
}

impl<'a> PartialOrd for HassStr<'a> {
	fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl<'a> Hash for HassStr<'a> {
	fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
		self.as_ref().hash(state)
	}
}

impl<'a> From<&'a str> for HassStr<'a> {
	#[inline]
	fn from(value: &'a str) -> Self {
		Self::Borrowed(value)
	}
}

#[cfg(feature = "alloc")]
impl<'a> From<Arc<str>> for HassStr<'a> {
	#[inline]
	fn from(value: Arc<str>) -> Self {
		Self::Arc(value)
	}
}

#[cfg(feature = "alloc")]
impl<'a> From<String> for HassStr<'a> {
	#[inline]
	fn from(value: String) -> Self {
		Self::Arc(value.into())
	}
}

impl<'a> AsRef<str> for HassStr<'a> {
	fn as_ref(&self) -> &str {
		match self {
			Self::Borrowed(s) => s,
			#[cfg(feature = "alloc")]
			Self::Arc(s) => s,
		}
	}
}

impl<'a> core::fmt::Debug for HassStr<'a> {
	#[inline]
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		core::fmt::Debug::fmt(&**self, f)
	}
}

impl<'a> core::fmt::Display for HassStr<'a> {
	#[inline]
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		core::fmt::Display::fmt(&**self, f)
	}
}

impl<'a> core::ops::Deref for HassStr<'a> {
	type Target = str;

	#[inline]
	fn deref(&self) -> &Self::Target {
		self.as_ref()
	}
}

#[cfg(feature = "ser")]
impl<'a> serde::Serialize for HassStr<'a> {
	#[inline]
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		match self {
			Self::Borrowed(s) => serializer.serialize_str(s),
			#[cfg(feature = "alloc")]
			Self::Arc(s) => serializer.serialize_str(s),
		}
	}
}

#[cfg(feature = "de")]
impl<'a, 'de: 'a> serde::Deserialize<'de> for HassStr<'a> {
	#[inline]
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		deserializer.deserialize_str(HassStrVisitor(PhantomData))
	}
}

#[cfg(feature = "de")]
impl<'de> serde::de::Visitor<'de> for HassStrVisitor<'de> {
	type Value = HassStr<'de>;

	fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
		formatter.write_str("a string")
	}

	// Borrowed directly from the input string, which has lifetime 'de
	// The input must outlive the resulting Cow.
	fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		Ok(HassStr::from(value))
	}

	// A string that currently only lives in a temporary buffer -- we need a copy
	// (Example: serde is reading from a BufRead)
	fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		Ok(HassStr::from(Arc::from(value)))
	}

	// An optimisation of visit_str for situations where the deserializer has
	// already taken ownership. For example, the string contains escaped characters.
	fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		Ok(HassStr::from(value))
	}
}

/// Does the heavy lifting of visiting borrowed strings
#[cfg(feature = "de")]
struct HassStrVisitor<'de>(PhantomData<&'de ()>);

macro_rules! typed_str {
  ($(#[$meta:meta])* $vis:vis $name:ident) => {
    $(#[$meta])*
    #[derive(Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
    $vis struct $name<'a>(pub(crate) HassStr<'a>);

		impl<'a> $name<'a> {
			#[cfg(feature = "alloc")]
			pub fn into_owned(self) -> $name<'static> {
				match self.0 {
					HassStr::Borrowed(s) => $name(HassStr::Arc(Arc::from(s))),
					HassStr::Arc(s) => $name(HassStr::Arc(s)),
				}
			}
		}

    impl<'a> From<HassStr<'a>> for $name<'a> {
      #[inline]
      fn from(value: HassStr<'a>) -> Self {
        Self(value)
      }
    }

		impl<'a> From<&'a str> for $name<'a> {
      #[inline]
      fn from(value: &'a str) -> Self {
        Self(HassStr::from(value))
      }
    }

		#[cfg(feature = "alloc")]
		impl<'a> From<&'a Arc<str>> for $name<'a> {
      #[inline]
      fn from(value: &'a Arc<str>) -> Self {
        Self::from(value.clone())
      }
    }

		#[cfg(feature = "alloc")]
    impl From<String> for $name<'_> {
      #[inline]
      fn from(value: String) -> Self {
        Self(HassStr::from(value))
      }
    }

		#[cfg(feature = "alloc")]
		impl From<Arc<str>> for $name<'_> {
      #[inline]
      fn from(value: Arc<str>) -> Self {
        Self(HassStr::from(value))
      }
    }

    impl<'a> core::fmt::Debug for $name<'a> {
      #[inline]
      fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&**self, f)
      }
    }

    impl<'a> core::fmt::Display for $name<'a> {
      #[inline]
      fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&**self, f)
      }
    }

    impl<'a> core::borrow::Borrow<str> for $name<'a> {
      #[inline]
      fn borrow(&self) -> &str {
        &**self
      }
    }

    impl<'a> AsRef<str> for $name<'a> {
      #[inline]
      fn as_ref(&self) -> &str {
        &**self
      }
    }

    impl<'a> core::ops::Deref for $name<'a> {
      type Target = str;

      fn deref(&self) -> &Self::Target {
        &*self.0
      }
    }

		#[cfg(feature = "ser")]
    impl<'a> serde::Serialize for $name<'a> {
      #[inline]
      fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
      where
        S: serde::Serializer,
      {
        self.0.serialize(serializer)
      }
    }

		#[cfg(feature = "de")]
    impl<'a, 'de: 'a> serde::Deserialize<'de> for $name<'a> {
      #[inline]
      fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
      where
        D: serde::Deserializer<'de>,
      {
				HassStr::deserialize(deserializer).map(Self)
      }
    }
  };
}

typed_str!(
	/// MQTT Topic name.
	pub Topic
);

typed_str!(
	/// Message payload.
	pub Payload
);

typed_str!(
	/// [Home-Assistant device icon][icon].
	///
	/// [icon]: https://www.home-assistant.io/docs/configuration/customizing-devices/#icon
	pub Icon
);

typed_str!(
	/// [Home-Assistant template][template].
	///
	/// [template]: https://www.home-assistant.io/docs/configuration/templating/
	pub Template
);

typed_str!(
	/// A device/entity name.
	pub Name
);

typed_str!(
	/// An ID that uniquely identifies this sensor. If two sensors have the same unique ID,
	/// Home Assistant will raise an exception..
	pub UniqueId
);

#[cfg(test)]
#[cfg(all(feature = "ser", feature = "de"))]
mod tests {
	use super::*;
	use assert_matches::assert_matches;
	use serde_test::{assert_tokens, Token};

	#[test]
	fn topic_ser_de() {
		assert_tokens(&Topic(HassStr::Borrowed("test")), &[Token::Str("test")])
	}

	#[test]
	fn topic_ser_de_borrowed() {
		let json = r#""test""#;
		let topic: Topic = serde_json::from_str(json).expect("should parse");
		assert_matches!(topic.0, HassStr::Borrowed(_));
	}

	#[test]
	fn topic_ser_de_escaped() {
		let json = r#""\test""#;
		let topic: Topic = serde_json::from_str(json).expect("should parse");
		assert_matches!(topic.0, HassStr::Arc(_));
	}
}
