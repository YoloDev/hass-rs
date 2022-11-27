use crate::{device_class::DeviceClass, payload::Payload, template::Template, topic::Topic};
use hass_mqtt_discovery_macros::entity_document;

/// The mqtt button platform lets you send an MQTT message when the button is
/// pressed in the frontend or the button press service is called.
/// This can be used to expose some service of a remote device, for example reboot.
///
/// See: <https://www.home-assistant.io/integrations/button.mqtt/>
#[entity_document]
pub struct Button<'a> {
	/// Defines a [template][template] to generate the payload to send to `command_topic`.
	///
	/// [template]: https://www.home-assistant.io/docs/configuration/templating/#using-templates-with-the-mqtt-integration
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub command_template: Option<Template<'a>>,

	/// The MQTT topic to publish commands to trigger the button.
	#[serde(borrow)]
	pub command_topic: Topic<'a>,

	/// The [type/class][device_class] of the button to set the icon in the frontend.
	///
	/// [device_class]: https://www.home-assistant.io/integrations/button/#device-class
	#[serde(default, skip_serializing_if = "DeviceClass::is_none")]
	pub device_class: DeviceClass,

	/// The payload to send to trigger the button.
	/// Defaults to `"PRESS"`.
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub payload_press: Option<Payload<'a>>,

	/// If the published message should have the retain flag on or not.
	/// Defaults to `false`.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub retain: Option<bool>,
}
