use crate::{
	device_tracker_source_type::DeviceTrackerSourceType, payload::Payload, template::Template,
	topic::Topic,
};
use hass_mqtt_discovery_macros::entity_document;

/// The mqtt device tracker platform allows you to automatically discover device_trackers
/// using the MQTT Discovery protocol.
///
/// See: <https://www.home-assistant.io/integrations/device_tracker.mqtt/>
#[entity_document]
pub struct DeviceTracker<'a> {
	/// The payload value that represents the `home` state for the device.
	/// Defaults to `"home"`.
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub payload_home: Option<Payload<'a>>,

	/// The payload value that represents the `not_home` state for the device.
	/// Defaults to `"not_home"`.
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub payload_not_home: Option<Payload<'a>>,

	/// Attribute of a device tracker that affects state when being used to track a [person][person].
	/// Valid options are `gps`, `router`, `bluetooth`, or `bluetooth_le`.
	///
	/// [person]: https://www.home-assistant.io/integrations/person/
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub source_type: Option<DeviceTrackerSourceType>,

	/// The MQTT topic subscribed to receive device tracker state changes.
	#[serde(borrow)]
	pub state_topic: Topic<'a>,

	/// Defines a [template][template] that returns a device tracker state.
	///
	/// [template]: https://www.home-assistant.io/docs/configuration/templating/#processing-incoming-data
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub value_template: Option<Template<'a>>,
}
