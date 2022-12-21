use color_eyre::{eyre::Context, Result};
use futures::StreamExt;
use hass_mqtt_client::{
	proto::{entity::LightState, Light},
	HassMqttOptions, Message, QosLevel,
};
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
async fn main() -> Result<()> {
	color_eyre::install()?;
	tracing_forest::init();

	println!("creating client");
	let client = HassMqttOptions::new("localhost", "mqtt-light")
		.node_id("1")
		.build_paho()
		.await
		.wrap_err(ApplicationError::CreateClient)?;

	println!("creating entity");
	let light_entity = client
		.entity("light", "mqtt_light")
		.await
		.wrap_err(ApplicationError::CreateEntity)?;

	println!("creating command topic");
	let mut command_topic = light_entity
		.command_topic()
		.qos(QosLevel::AtLeastOnce)
		.await
		.wrap_err(ApplicationError::CommandTopic)?;

	println!("creating state topic");
	let state_topic = light_entity
		.state_topic()
		.await
		.wrap_err(ApplicationError::StateTopic)?;

	let light_discovery_document = Light::new(&command_topic)
		.object_id("mqtt_light")
		.name("MQTT Light")
		.state_topic(&state_topic);
	let light_discovery_document = serde_json::to_vec(&light_discovery_document)
		.wrap_err(ApplicationError::SerializeDiscoveryDocument)?;

	println!("publishing discovery document");
	light_entity
		.publish(light_discovery_document, true, QosLevel::AtLeastOnce)
		.await
		.wrap_err(ApplicationError::PublishDiscoveryDocument)?;

	let mut on = false;
	let autoflip_duration = Duration::from_secs(5);

	loop {
		let state_doc = serde_json::to_vec(&LightState::new(on))
			.wrap_err(ApplicationError::SerializeStateDocument)?;

		println!("publishing state document");
		state_topic
			.publish(state_doc, true, QosLevel::AtLeastOnce)
			.await
			.wrap_err(ApplicationError::PublishStateDocument)?;

		select! {
			Some(cmd) = command_topic.next() => on = parse(cmd)?,
			_ = sleep(autoflip_duration) => on = !on,
			else => break,
		};
	}

	Ok(())
}

fn parse(cmd: Message) -> Result<bool> {
	let state: LightState =
		serde_json::from_slice(cmd.payload()).wrap_err(ApplicationError::ParseCommand)?;

	Ok(state.state.is_on())
}
