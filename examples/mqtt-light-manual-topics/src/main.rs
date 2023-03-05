use error_stack::{IntoReport, ResultExt};
use futures::StreamExt;
use hass_mqtt_client::{
	proto::{entity::LightState, Light},
	HassMqttOptions, Message, QosLevel,
};
use opentelemetry::{sdk::Resource, Key};
use prometheus::{Encoder, TextEncoder};
use std::time::Duration;
use thiserror::Error;
use tokio::{select, time::sleep};
use tracing_subscriber::{prelude::*, Registry};
use tracing_tree::HierarchicalLayer;

#[derive(Debug, Error)]
enum ApplicationError {
	#[error("setup opentelemetry")]
	SetupOtel,

	#[error("prometheus error")]
	Prometheus,

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
	opentelemetry::global::set_text_map_propagator(opentelemetry_jaeger::Propagator::new());
	let tracer = opentelemetry_jaeger::new_agent_pipeline()
		.with_service_name(env!("CARGO_PKG_NAME"))
		.install_batch(opentelemetry::runtime::Tokio)
		.into_report()
		.change_context(ApplicationError::SetupOtel)?;

	let metrics_controller = opentelemetry::sdk::metrics::controllers::basic(
		opentelemetry::sdk::metrics::processors::factory(
			opentelemetry::sdk::metrics::selectors::simple::histogram([1.0, 2.0, 5.0, 10.0, 20.0, 50.0]),
			opentelemetry::sdk::export::metrics::aggregation::cumulative_temporality_selector(),
		)
		.with_memory(true),
	)
	.with_resource(Resource::new(vec![
		Key::new("service.name").string(env!("CARGO_PKG_NAME"))
	]))
	.build();
	let exporter = opentelemetry_prometheus::exporter(metrics_controller.clone()).init();

	let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
	let metrics = tracing_opentelemetry::MetricsLayer::new(metrics_controller);
	Registry::default()
		.with(HierarchicalLayer::new(2))
		.with(telemetry)
		.with(metrics)
		.init();

	// const UNIQUE_ID: &str = "0xabcdef0123456789";
	const ROOT: &str = "homeassistant/light/0xabcdef0123456789";

	println!("creating client");
	let client = HassMqttOptions::new("localhost", "mqtt-light")
		.node_id("2")
		.build_paho()
		.await
		.into_report()
		.change_context(ApplicationError::CreateClient)?;

	println!("creating entity");
	let light_entity = client
		.entity("light", "mqtt_light")
		.with_topic(format!("{ROOT}/config"))
		.await
		.into_report()
		.change_context(ApplicationError::CreateEntity)?;

	println!("creating command topic");
	let mut command_topic = light_entity
		.command_topic()
		.topic(format!("{ROOT}/set"))
		.qos(QosLevel::AtLeastOnce)
		.await
		.into_report()
		.change_context(ApplicationError::CommandTopic)?;

	println!("creating state topic");
	let state_topic = light_entity
		.state_topic()
		.topic(format!("{ROOT}/state"))
		.await
		.into_report()
		.change_context(ApplicationError::StateTopic)?;

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

		// Encode data as text or protobuf
		let encoder = TextEncoder::new();
		let metric_families = exporter.registry().gather();
		let mut result = Vec::new();
		encoder
			.encode(&metric_families, &mut result)
			.into_report()
			.change_context(ApplicationError::Prometheus)?;

		println!("metrics: {}", String::from_utf8_lossy(&result));

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

#[tracing::instrument(skip(cmd), parent = cmd.span())]
fn parse(cmd: Message) -> error_stack::Result<bool, ApplicationError> {
	let state: LightState = serde_json::from_slice(cmd.payload())
		.into_report()
		.change_context(ApplicationError::ParseCommand)?;

	Ok(state.state.is_on())
}
