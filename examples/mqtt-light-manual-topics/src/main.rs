use error_stack::ResultExt;
use futures::StreamExt;
use hass_mqtt_client::{
	proto::{entity::LightState, Light},
	HassMqttOptions, QosLevel,
};
use std::time::Duration;
use thiserror::Error;
use tokio::{select, time::sleep};
use tracing::instrument;

#[derive(Debug, Error)]
enum ApplicationError {
	#[error("create client")]
	CreateClient,

	#[error("create entity")]
	CreateEntity,

	#[error("command topic")]
	CommandTopic,

	#[error("state topic")]
	StateTopic,

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
	// const UNIQUE_ID: &str = "0xabcdef0123456789";
	const ROOT: &str = "homeassistant/light/0xabcdef0123456789";

	println!("creating client");
	let client = HassMqttOptions::new("localhost", "mqtt-light")
		.mqtt_v5()
		.node_id("2")
		.build_paho()
		.await
		.change_context(ApplicationError::CreateClient)?;

	println!("creating entity");
	let light_entity = client
		.entity("light", "mqtt_light")
		.with_topic(format!("{ROOT}/config"))
		.await
		.change_context(ApplicationError::CreateEntity)?;

	println!("creating command topic");
	let mut command_topic = light_entity
		.command_topic()
		.topic(format!("{ROOT}/set"))
		.qos(QosLevel::AtLeastOnce)
		.await
		.change_context(ApplicationError::CommandTopic)?;

	println!("creating state topic");
	let state_topic = light_entity
		.state_topic()
		.topic(format!("{ROOT}/state"))
		.await
		.change_context(ApplicationError::StateTopic)?;

	let light_discovery_document = Light::new(&command_topic)
		.object_id("mqtt_light")
		.name("MQTT Light")
		.state_topic(&state_topic);
	let light_discovery_document = serde_json::to_vec(&light_discovery_document)
		.change_context(ApplicationError::SerializeDiscoveryDocument)?;

	println!("publishing discovery document");
	light_entity
		.publish(light_discovery_document, true, QosLevel::AtLeastOnce)
		.await
		.change_context(ApplicationError::PublishDiscoveryDocument)?;

	let mut on = false;
	let autoflip_duration = Duration::from_secs(5);

	loop {
		let state_doc = serde_json::to_vec(&LightState::new(on))
			.change_context(ApplicationError::SerializeStateDocument)?;

		println!("publishing state document");
		state_topic
			.publish(state_doc, true, QosLevel::AtLeastOnce)
			.await
			.change_context(ApplicationError::PublishStateDocument)?;

		select! {
			Some(cmd) = command_topic.next() => {
				let _entered = cmd.span().enter();
				on = parse(cmd.payload())?;
			},
			_ = sleep(autoflip_duration) => on = !on,
			else => break,
		};
	}

	Ok(())
}

#[instrument(skip_all)]
fn parse(payload: &[u8]) -> error_stack::Result<bool, ApplicationError> {
	let state: LightState =
		serde_json::from_slice(payload).change_context(ApplicationError::ParseCommand)?;

	Ok(state.state.is_on())
}
