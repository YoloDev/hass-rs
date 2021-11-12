use crate::{
  exts::ValidateContextExt,
  payload::{Payload, PayloadInvalidity},
  topic::{Topic, TopicInvalidity},
};
use semval::{context::Context, Validate};
use serde::{Deserialize, Serialize};

pub enum AvailabilityMode {
  /// If set to all, `payload_available` must be received on all configured availability topics before the entity is marked as online.
  All,

  /// If set to any, `payload_available` must be received on at least one configured availability topic before the entity is marked as online.
  Any,

  /// If set to latest, the last `payload_available` or `payload_not_available` received on any configured availability topic controls the availability.
  Latest,
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
          name: "Availability",
          len: 1,
        },
        Token::Str("topic"),
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
          name: "Availability",
          len: 3,
        },
        Token::Str("topic"),
        Token::Str("the/topic"),
        Token::Str("payload_available"),
        Token::Some,
        Token::Str("available"),
        Token::Str("payload_not_available"),
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
    assert!(matches!(availability.topic, Topic(Cow::Borrowed(_))));
  }
}
