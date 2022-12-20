#![cfg_attr(provide_any, feature(provide_any))]
#![cfg_attr(provide_any, feature(error_generic_member_access))]

mod availability;
mod client;
mod entity;
mod mqtt;
mod options;
mod router;
mod topics;
mod tracking;

pub use client::{ConnectError, HassMqttClient, Message};
pub use entity::{
	CommandTopic, CommandTopicBuilder, CreateEntityError, EntityPublishError, EntitySubscribeError,
	EntityTopic, EntityTopicBuilder, StateTopic, StateTopicBuilder,
};
pub use hass_mqtt_proto as proto;
pub use hass_mqtt_provider::QosLevel;
pub use options::{HassMqttOptions, MqttOptionsError, MqttPersistenceError};
