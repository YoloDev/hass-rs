use error_stack::{IntoReport, ResultExt};
use hass_mqtt_client::{HassMqttOptions, MqttQosLevel};
use hass_mqtt_types::Light;
use thiserror::Error;

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
}

#[tokio::main]
async fn main() -> error_stack::Result<(), ApplicationError> {
	println!("creating client");
	let client = HassMqttOptions::new("localhost", "mqtt-light")
		.build_paho()
		.await
		.change_context(ApplicationError::CreateClient)?;

	println!("creating entity");
	let light_entity = client
		.entity("light", "mqtt_light")
		.await
		.change_context(ApplicationError::CreateEntity)?;

	println!("creating command topic");
	let command_topic = light_entity
		.command_topic("set", MqttQosLevel::AtLeastOnce)
		.await
		.change_context(ApplicationError::CommandTopic)?;
	let topic = command_topic.topic();
	let light_discovery_document = Light::new(&*topic)
		.object_id("mqtt_light")
		.name("MQTT Light");
	let light_discovery_document = serde_json::to_vec(&light_discovery_document)
		.into_report()
		.change_context(ApplicationError::SerializeDiscoveryDocument)?;

	println!("publishing discovery document");
	light_entity
		.publish(light_discovery_document, true, MqttQosLevel::AtLeastOnce)
		.await
		.change_context(ApplicationError::PublishDiscoveryDocument)?;

	Ok(())
}
