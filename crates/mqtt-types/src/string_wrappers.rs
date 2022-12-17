use std::sync::Arc;

/// Does the heavy lifting of visiting borrowed strings
struct TypedStrVisitor<T>(std::marker::PhantomData<T>);

macro_rules! typed_str {
  ($(#[$meta:meta])* $vis:vis $name:ident) => {
    $(#[$meta])*
    #[derive(Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
    $vis struct $name<'a>(pub(crate) std::borrow::Cow<'a, str>);

    impl<'a> From<std::borrow::Cow<'a, str>> for $name<'a> {
      #[inline]
      fn from(value: std::borrow::Cow<'a, str>) -> Self {
        Self(value)
      }
    }

    impl<'a> From<&'a str> for $name<'a> {
      #[inline]
      fn from(value: &'a str) -> Self {
        Self(std::borrow::Cow::Borrowed(value))
      }
    }

		impl<'a> From<&'a Arc<str>> for $name<'a> {
      #[inline]
      fn from(value: &'a Arc<str>) -> Self {
        Self::from(&**value)
      }
    }

    impl From<String> for $name<'_> {
      #[inline]
      fn from(value: String) -> Self {
        Self(std::borrow::Cow::Owned(value))
      }
    }

		impl From<Arc<str>> for $name<'_> {
      #[inline]
      fn from(value: Arc<str>) -> Self {
        Self::from(ToOwned::to_owned(&*value))
      }
    }

    impl<'a> std::fmt::Debug for $name<'a> {
      #[inline]
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&**self, f)
      }
    }

    impl<'a> std::fmt::Display for $name<'a> {
      #[inline]
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&**self, f)
      }
    }

    impl<'a> std::borrow::Borrow<str> for $name<'a> {
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

    impl<'a> std::ops::Deref for $name<'a> {
      type Target = str;

      fn deref(&self) -> &Self::Target {
        &*self.0
      }
    }

    impl<'a> serde::Serialize for $name<'a> {
      #[inline]
      fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
      where
        S: serde::Serializer,
      {
        self.0.serialize(serializer)
      }
    }

    impl<'a, 'de: 'a> serde::de::Visitor<'de> for TypedStrVisitor<$name<'a>> {
      type Value = $name<'a>;

      fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a string")
      }

      // Borrowed directly from the input string, which has lifetime 'de
      // The input must outlive the resulting Cow.
      fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
      where
        E: serde::de::Error,
      {
        Ok($name(std::borrow::Cow::Borrowed(value)))
      }

      // A string that currently only lives in a temporary buffer -- we need a copy
      // (Example: serde is reading from a BufRead)
      fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
      where
        E: serde::de::Error,
      {
        Ok($name(std::borrow::Cow::Owned(value.to_owned())))
      }

      // An optimisation of visit_str for situations where the deserializer has
      // already taken ownership. For example, the string contains escaped characters.
      fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
      where
        E: serde::de::Error,
      {
        Ok($name(std::borrow::Cow::Owned(value)))
      }
    }

    impl<'a, 'de: 'a> serde::Deserialize<'de> for $name<'a> {
      fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
      where
        D: serde::Deserializer<'de>,
      {
        deserializer.deserialize_str(TypedStrVisitor::<$name>(std::marker::PhantomData))
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
mod tests {
	use super::*;
	use assert_matches::assert_matches;
	use serde_test::{assert_tokens, Token};
	use std::borrow::Cow;

	#[test]
	fn topic_ser_de() {
		assert_tokens(&Topic(Cow::Borrowed("test")), &[Token::Str("test")])
	}

	#[test]
	fn topic_ser_de_borrowed() {
		let json = r#""test""#;
		let topic: Topic = serde_json::from_str(json).expect("should parse");
		assert_matches!(topic.0, Cow::Borrowed(_));
	}

	#[test]
	fn topic_ser_de_escaped() {
		let json = r#""\test""#;
		let topic: Topic = serde_json::from_str(json).expect("should parse");
		assert_matches!(topic.0, Cow::Owned(_));
	}
}
