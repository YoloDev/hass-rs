use crate::{
  exts::ValidateContextExt,
  payload::{Payload, PayloadInvalidity},
  topic::{Topic, TopicInvalidity},
};
use semval::{context::Context, Validate};
use serde::{Deserialize, Serialize};

/// When availability is configured, this controls the conditions needed to set the entity to available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AvailabilityMode {
  /// If set to all, `payload_available` must be received on all configured availability topics before the entity is marked as online.
  #[serde(rename = "all")]
  All,

  /// If set to any, `payload_available` must be received on at least one configured availability topic before the entity is marked as online.
  #[serde(rename = "any")]
  Any,

  /// If set to latest, the last `payload_available` or `payload_not_available` received on any configured availability topic controls the availability.
  ///
  /// This is the default mode if not specified.
  #[serde(rename = "latest")]
  Latest,
}

impl AvailabilityMode {
  #[inline]
  pub const fn is_default(&self) -> bool {
    matches!(self, Self::Latest)
  }
}

impl Default for AvailabilityMode {
  #[inline]
  fn default() -> Self {
    Self::Latest
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Availability<'a> {
  /// An MQTT topic subscribed to receive availability (online/offline) updates.
  #[serde(borrow)]
  pub topic: Topic<'a>,

  /// The payload that represents the available state.
  ///
  /// The default (used if `None`) is `online`.
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub payload_available: Option<Payload<'a>>,

  /// The payload that represents the unavailable state.
  ///
  /// The default (used if `None`) is `offline`.
  #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
  pub payload_not_available: Option<Payload<'a>>,
}

impl<'a> Availability<'a> {
  pub fn new(topic: impl Into<Topic<'a>>) -> Self {
    Self {
      topic: topic.into(),
      payload_available: None,
      payload_not_available: None,
    }
  }

  pub fn new_with_payloads(
    topic: impl Into<Topic<'a>>,
    available_payload: impl Into<Payload<'a>>,
    not_available_payload: impl Into<Payload<'a>>,
  ) -> Self {
    Self {
      topic: topic.into(),
      payload_available: Some(available_payload.into()),
      payload_not_available: Some(not_available_payload.into()),
    }
  }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AvailabilityDataInvalidity {
  Topic(TopicInvalidity),
  PayloadAvailable(PayloadInvalidity),
  PayloadNotAvailable(PayloadInvalidity),
}

impl<'a> Validate for Availability<'a> {
  type Invalidity = AvailabilityDataInvalidity;

  fn validate(&self) -> semval::Result<Self::Invalidity> {
    Context::new()
      .validate_with(&self.topic, AvailabilityDataInvalidity::Topic)
      .validate_with_opt(
        &self.payload_available,
        AvailabilityDataInvalidity::PayloadAvailable,
      )
      .validate_with_opt(
        &self.payload_not_available,
        AvailabilityDataInvalidity::PayloadNotAvailable,
      )
      .into()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use assert_matches::assert_matches;
  use nameof::{name_of, name_of_type};
  use serde_test::{assert_tokens, Token};
  use std::borrow::Cow;

  #[test]
  fn no_payloads() {
    assert_tokens(
      &Availability {
        topic: Topic(Cow::Borrowed("the/topic")),
        payload_available: None,
        payload_not_available: None,
      },
      &[
        Token::Struct {
          name: name_of_type!(Availability),
          len: 1,
        },
        Token::Str(name_of!(topic in Availability)),
        Token::Str("the/topic"),
        Token::StructEnd,
      ],
    )
  }

  #[test]
  fn with_payloads() {
    assert_tokens(
      &Availability {
        topic: Topic(Cow::Borrowed("the/topic")),
        payload_available: Some(Payload(Cow::Borrowed("available"))),
        payload_not_available: Some(Payload(Cow::Borrowed("not_available"))),
      },
      &[
        Token::Struct {
          name: name_of_type!(Availability),
          len: 3,
        },
        Token::Str(name_of!(topic in Availability)),
        Token::Str("the/topic"),
        Token::Str(name_of!(payload_available in Availability)),
        Token::Some,
        Token::Str("available"),
        Token::Str(name_of!(payload_not_available in Availability)),
        Token::Some,
        Token::Str("not_available"),
        Token::StructEnd,
      ],
    )
  }

  #[test]
  fn deserialize_json_borrows() {
    let json = r##"{"topic":"the/topic"}"##;
    let availability: Availability = serde_json::from_str(json).expect("should parse");
    assert_matches!(availability.topic, Topic(Cow::Borrowed(_)));
  }

  #[test]
  fn invalid_payload_available_is_invalid() {
    let err: Vec<_> = Availability {
      topic: Topic::from("topic"),
      payload_available: Some(Payload::from("")),
      payload_not_available: None,
    }
    .validate()
    .expect_err("should be invalid")
    .into_iter()
    .collect();

    assert_eq!(
      &*err,
      &[AvailabilityDataInvalidity::PayloadAvailable(
        PayloadInvalidity::Empty
      )]
    )
  }

  #[test]
  fn invalid_payload_not_available_is_invalid() {
    let err: Vec<_> = Availability {
      topic: Topic::from("topic"),
      payload_available: None,
      payload_not_available: Some(Payload::from("")),
    }
    .validate()
    .expect_err("should be invalid")
    .into_iter()
    .collect();

    assert_eq!(
      &*err,
      &[AvailabilityDataInvalidity::PayloadNotAvailable(
        PayloadInvalidity::Empty
      )]
    )
  }

  #[test]
  fn invalid_topic_is_invalid() {
    let err: Vec<_> = Availability {
      topic: Topic::from(""),
      payload_available: None,
      payload_not_available: None,
    }
    .validate()
    .expect_err("should be invalid")
    .into_iter()
    .collect();

    assert_eq!(
      &*err,
      &[AvailabilityDataInvalidity::Topic(TopicInvalidity::Empty)]
    )
  }
}
