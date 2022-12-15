use error_stack::{IntoReport, ResultExt};
use futures::StreamExt;
use hass_mqtt_client::{HassMqttOptions, Message, QosLevel};
use hass_mqtt_types::{entity::LightState, Light};
use std::time::Duration;
use thiserror::Error;
use tokio::{select, time::sleep};

#[derive(Debug, Error)]
enum ApplicationError {
	#[error("create client")]
	CreateClient,

	#[error("create entity")]
	CreateEntity,

	#[error("command topic")]
	CommandTopic,

	#[error("serialize discovery document")]
	SerializeDiscoveryDocument,

	#[error("publish discovery document")]
	PublishDiscoveryDocument,

	#[error("serialize state document")]
	SerializeStateDocument,

	#[error("publish state document")]
	PublishStateDocument,

	#[error("parse command")]
	ParseCommand,
}

#[tokio::main]
async fn main() -> error_stack::Result<(), ApplicationError> {
	println!("creating client");
	let client = HassMqttOptions::new("localhost", "mqtt-light")
		.build_paho()
		.await
		.into_report()
		.change_context(ApplicationError::CreateClient)?;

	println!("creating entity");
	let light_entity = client
		.entity("light", "mqtt_light")
		.await
		.into_report()
		.change_context(ApplicationError::CreateEntity)?;

	println!("creating command topic");
	let mut command_topic = light_entity
		.command_topic("set", QosLevel::AtLeastOnce)
		.await
		.into_report()
		.change_context(ApplicationError::CommandTopic)?;

	println!("creating state topic");
	let state_topic = light_entity.state_topic("json");

	let light_discovery_document = Light::new(&command_topic)
		.object_id("mqtt_light")
		.name("MQTT Light")
		.state_topic(&state_topic);
	let light_discovery_document = serde_json::to_vec(&light_discovery_document)
		.into_report()
		.change_context(ApplicationError::SerializeDiscoveryDocument)?;

	println!("publishing discovery document");
	light_entity
		.publish(light_discovery_document, true, QosLevel::AtLeastOnce)
		.await
		.into_report()
		.change_context(ApplicationError::PublishDiscoveryDocument)?;

	let mut on = false;
	let autoflip_duration = Duration::from_secs(5);

	loop {
		let state_doc = serde_json::to_vec(&LightState::new(on))
			.into_report()
			.change_context(ApplicationError::SerializeStateDocument)?;

		println!("publishing state document");
		state_topic
			.publish(state_doc, true, QosLevel::AtLeastOnce)
			.await
			.into_report()
			.change_context(ApplicationError::PublishStateDocument)?;

		select! {
			Some(cmd) = command_topic.next() => on = parse(cmd)?,
			_ = sleep(autoflip_duration) => on = !on,
			else => break,
		};
	}

	Ok(())
}

fn parse(cmd: Message) -> error_stack::Result<bool, ApplicationError> {
	let state: LightState = serde_json::from_slice(cmd.payload())
		.into_report()
		.change_context(ApplicationError::ParseCommand)?;

	Ok(state.state.is_on())
}
