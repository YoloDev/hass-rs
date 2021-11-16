use super::{AttributeKey, AttributeValue, Attributes};
use crate::EntityState;
use serde::{de::MapAccess, ser::SerializeMap, Deserialize, Serialize};
use std::{collections::BTreeMap, fmt, marker::PhantomData};

impl<'a, K, V, E> Serialize for Attributes<'a, K, V, E>
where
  K: AttributeKey<'a>,
  V: AttributeValue<'a>,
  E: EntityState<'a, K, V>,
{
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let mut map = serializer.serialize_map(Some(self.0.len()))?;
    for (key, value) in self.0.iter() {
      map.serialize_entry(key.borrow(), value.borrow())?;
    }
    map.end()
  }
}

struct AttributesVisitor<'a, K, V, E>
where
  K: AttributeKey<'a>,
  V: AttributeValue<'a>,
  E: EntityState<'a, K, V>,
{
  marker: PhantomData<Attributes<'a, K, V, E>>,
}

impl<'a, 'de: 'a, K, V, E> serde::de::Visitor<'de> for AttributesVisitor<'a, K, V, E>
where
  K: AttributeKey<'a> + 'a,
  V: AttributeValue<'a> + 'a,
  E: EntityState<'a, K, V>,
{
  type Value = Attributes<'a, K, V, E>;

  fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
    formatter.write_str("a map")
  }

  #[inline]
  fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
  where
    A: MapAccess<'de>,
  {
    // TODO: Object pool?
    let mut values = BTreeMap::new();

    while let Some((AttributeKeyWrapper { value: key, .. }, AttributeValueWrapper { value, .. })) =
      map.next_entry()?
    {
      values.insert(key, value);
    }

    Ok(Attributes(values, PhantomData))
  }
}

impl<'a, 'de: 'a, K, V, E> Deserialize<'de> for Attributes<'a, K, V, E>
where
  K: AttributeKey<'a> + 'a,
  V: AttributeValue<'a> + 'a,
  E: EntityState<'a, K, V>,
{
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    deserializer.deserialize_map(AttributesVisitor {
      marker: PhantomData,
    })
  }
}

struct AttributeKeyWrapper<'a, K> {
  value: K,
  marker: PhantomData<&'a ()>,
}
struct AttributeKeyVisitor<'a, K> {
  marker: PhantomData<fn() -> &'a K>,
}
impl<'a, 'de: 'a, K> serde::de::Visitor<'de> for AttributeKeyVisitor<'a, K>
where
  K: AttributeKey<'a>,
{
  type Value = AttributeKeyWrapper<'a, K>;

  fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    formatter.write_str("a string")
  }

  // Borrowed directly from the input string, which has lifetime 'de
  // The input must outlive the resulting Cow.
  fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    Ok(AttributeKeyWrapper {
      value: K::from_cow(std::borrow::Cow::Borrowed(value)),
      marker: PhantomData,
    })
  }

  // A string that currently only lives in a temporary buffer -- we need a copy
  // (Example: serde is reading from a BufRead)
  fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    Ok(AttributeKeyWrapper {
      value: K::from_cow(std::borrow::Cow::Owned(value.to_owned())),
      marker: PhantomData,
    })
  }

  // An optimisation of visit_str for situations where the deserializer has
  // already taken ownership. For example, the string contains escaped characters.
  fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    Ok(AttributeKeyWrapper {
      value: K::from_cow(std::borrow::Cow::Owned(value)),
      marker: PhantomData,
    })
  }
}
impl<'a, 'de: 'a, K: 'a> Deserialize<'de> for AttributeKeyWrapper<'a, K>
where
  K: AttributeKey<'a>,
{
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    deserializer.deserialize_str(AttributeKeyVisitor {
      marker: PhantomData,
    })
  }
}

struct AttributeValueWrapper<'a, V> {
  value: V,
  marker: PhantomData<&'a ()>,
}
struct AttributeValueVisitor<'a, V> {
  marker: PhantomData<fn() -> &'a V>,
}
impl<'a, 'de: 'a, V> serde::de::Visitor<'de> for AttributeValueVisitor<'a, V>
where
  V: AttributeValue<'a>,
{
  type Value = AttributeValueWrapper<'a, V>;

  fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    formatter.write_str("a string")
  }

  // Borrowed directly from the input string, which has lifetime 'de
  // The input must outlive the resulting Cow.
  fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    Ok(AttributeValueWrapper {
      value: V::from_cow(std::borrow::Cow::Borrowed(value)),
      marker: PhantomData,
    })
  }

  // A string that currently only lives in a temporary buffer -- we need a copy
  // (Example: serde is reading from a BufRead)
  fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    Ok(AttributeValueWrapper {
      value: V::from_cow(std::borrow::Cow::Owned(value.to_owned())),
      marker: PhantomData,
    })
  }

  // An optimisation of visit_str for situations where the deserializer has
  // already taken ownership. For example, the string contains escaped characters.
  fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    Ok(AttributeValueWrapper {
      value: V::from_cow(std::borrow::Cow::Owned(value)),
      marker: PhantomData,
    })
  }
}

impl<'a, 'de: 'a, V: 'a> Deserialize<'de> for AttributeValueWrapper<'a, V>
where
  V: AttributeValue<'a>,
{
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    deserializer.deserialize_str(AttributeValueVisitor {
      marker: PhantomData,
    })
  }
}

#[cfg(test)]
mod tests {
  use std::borrow::Cow;

  use assert_matches::assert_matches;

  use crate::EntityStateValue;

  use super::*;
  use serde_test::{assert_tokens, Token};

  struct FakeEntityType;
  impl EntityStateValue for FakeEntityType {}
  impl<'a> EntityState<'a, Cow<'a, str>, Cow<'a, str>> for FakeEntityType {
    type State = FakeEntityType;

    fn get(&self) -> &Self::State {
      todo!()
    }

    fn get_mut(&mut self) -> &mut Self::State {
      todo!()
    }

    fn attributes(&self) -> &Attributes<'a, Cow<'a, str>, Cow<'a, str>, Self> {
      todo!()
    }

    fn attributes_mut(&mut self) -> &mut Attributes<'a, Cow<'a, str>, Cow<'a, str>, Self> {
      todo!()
    }
  }

  #[test]
  fn serde() {
    let mut attributes: Attributes<Cow<str>, Cow<str>, FakeEntityType> = Attributes::default();
    attributes.insert(Cow::Borrowed("k1"), Cow::Borrowed("v1"));
    attributes.insert(Cow::Borrowed("k2"), Cow::Borrowed("v2"));

    assert_tokens(
      &attributes,
      &[
        Token::Map { len: Some(2) },
        Token::Str("k1"),
        Token::Str("v1"),
        Token::Str("k2"),
        Token::Str("v2"),
        Token::MapEnd,
      ],
    )
  }

  #[test]
  fn json_borrows() {
    let json = r#"{"k1":"v1","k2":"v2"}"#;
    let attributes: Attributes<Cow<str>, Cow<str>, FakeEntityType> =
      serde_json::from_str(json).expect("should parse");

    assert_matches!(attributes.get_inner("k1"), Some(Cow::Borrowed("v1")));
    assert_matches!(attributes.get_inner("k2"), Some(Cow::Borrowed("v2")));
  }
}
