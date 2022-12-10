use crate::{
	device_class::DeviceClass, payload::Payload, template::Template, topic::Topic,
	validation::Validator,
};
use hass_mqtt_discovery_macros::entity_document;

/// The mqtt cover platform allows you to control an MQTT cover (such as blinds, a roller shutter or a garage door).
///
/// See: <https://www.home-assistant.io/integrations/cover.mqtt/>
#[entity_document]
#[entity(validate(
	StateTopicMissing,
	PositionTopicMissing,
	SetPositionTopicMissing,
	TiltCommandTopicMissing,
	TiltStatusTopicMissing
))]
pub struct Cover<'a> {
	/// The MQTT topic to publish commands to control the cover.
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub command_topic: Option<Topic<'a>>,

	/// Sets the [class of the device][device_class], changing the device state and icon that is displayed on the frontend.
	///
	/// [device_class]: https://www.home-assistant.io/integrations/cover/#device-class
	#[serde(default, skip_serializing_if = "DeviceClass::is_none")]
	pub device_class: DeviceClass,

	/// Flag that defines if the cover works in optimistic mode.
	/// Defaults to `false` if a `state_topic` or `position_topic` is defined, else `true`.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub optimistic: Option<bool>,

	/// The command payload that closes the cover.
	/// Defaults to `"CLOSE"`.
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub payload_close: Option<Payload<'a>>,

	/// The command payload that opens the cover.
	/// Defaults to `"OPEN"`.
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub payload_open: Option<Payload<'a>>,

	/// The command payload that stops the cover.
	/// Defaults to `"STOP"`.
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub payload_stop: Option<Payload<'a>>,

	/// Number which represents closed position.
	/// Defaults to `0`.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub position_closed: Option<u32>,

	/// Number which represents open position.
	/// Defaults to `100`.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub position_open: Option<u32>,

	/// Defines a [template][template] that can be used to extract the
	/// payload for the `position_topic` topic. Within the template the
	/// following variables are available:
	/// - `entity_id`
	/// - `position_open`
	/// - `position_closed`
	/// - `tilt_min`
	/// - `tilt_max`
	///
	/// The `entity_id` can be used to reference the entity’s attributes with
	/// the help of the [states][states] template function.
	///
	/// [template]: https://www.home-assistant.io/docs/configuration/templating/#processing-incoming-data
	/// [states]: https://www.home-assistant.io/docs/configuration/templating/#states
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub position_template: Option<Template<'a>>,

	/// The MQTT topic subscribed to receive cover position messages.
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub position_topic: Option<Topic<'a>>,

	/// Defines if published messages should have the retain flag set.
	/// Defaults to `false`.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub retain: Option<bool>,

	/// Defines a [template][template] to define the position to be sent
	/// to the `set_position_topic` topic. Incoming position value is
	/// available for use in the template `{{ position }}`. Within the
	/// template the following variables are available:
	/// - `entity_id`
	/// - `position` in percent
	/// - `position_open`
	/// - `position_closed`
	/// - `tilt_min`
	/// - `tilt_max`
	///
	/// The `entity_id` can be used to reference the entity’s attributes with
	/// the help of the [states][states] template function.
	///
	/// [template]: https://www.home-assistant.io/docs/configuration/templating/#processing-incoming-data
	/// [states]: https://www.home-assistant.io/docs/configuration/templating/#states
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub set_position_template: Option<Template<'a>>,

	/// The MQTT topic to publish position commands to.
	/// You need to set `position_topic` as well if you want to use position
	/// topic. Use template if position topic wants different values than within
	/// range `position_closed` - `position_open`. If template is not defined
	/// and `position_closed` != 100 and `position_open` != 0 then proper
	/// position value is calculated from percentage position.
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub set_position_topic: Option<Topic<'a>>,

	/// The payload that represents the closed state.
	/// Defaults to `"closed"`.
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub state_closed: Option<Payload<'a>>,

	/// The payload that represents the closing state.
	/// Defaults to `"closing"`.
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub state_closing: Option<Payload<'a>>,

	/// The payload that represents the open state.
	/// Defaults to `"open"`.
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub state_open: Option<Payload<'a>>,

	/// The payload that represents the opening state.
	/// Defaults to `"opening"`.
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub state_opening: Option<Payload<'a>>,

	/// The payload that represents the stopped state
	/// (for covers that do not report `open`/`closed` state).
	/// Defaults to `"stopped"`.
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub state_stopped: Option<Payload<'a>>,

	/// The MQTT topic subscribed to receive cover state messages.
	/// State topic can only read (`open`, `opening`, `closed`, `closing`
	/// or `stopped`) state.
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub state_topic: Option<Topic<'a>>,

	/// The value that will be sent on a close_cover_tilt command.
	/// Defaults to `0`.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub tilt_closed_value: Option<u32>,

	/// Defines a [template][template] that can be used to extract the
	/// payload for the `tilt_command_topic` topic. Within the template
	/// the following variables are available:
	/// - `entity_id`
	/// - `tilt_position` in percent
	/// - `position_open`
	/// - `position_closed`
	/// - `tilt_min`
	/// - `tilt_max`
	///
	/// The `entity_id` can be used to reference the entity’s attributes
	/// with the help of the [states][states] template function.
	///
	/// [template]: https://www.home-assistant.io/docs/configuration/templating/#processing-incoming-data
	/// [states]: https://www.home-assistant.io/docs/configuration/templating/#states
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub tilt_command_template: Option<Template<'a>>,

	/// The MQTT topic to publish commands to control the cover tilt.
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub tilt_command_topic: Option<Topic<'a>>,

	/// The maximum tilt value.
	/// Defaults to `100`.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub tilt_max: Option<u32>,

	/// The minimum tilt value.
	/// Defaults to `0`.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub tilt_min: Option<u32>,

	/// The value that will be sent on an `open_cover_tilt` command.
	/// Defaults to `100`.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub tilt_opened_value: Option<u32>,

	/// Flag that determines if tilt works in optimistic mode.
	/// Defaults to `true` if `tilt_status_topic` is not defined, else `false`.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub tile_optimistic: Option<bool>,

	/// Defines a [template][template] that can be used to extract the
	/// payload for the `tilt_status_topic` topic. Within the template
	/// the following variables are available:
	/// - `entity_id`
	/// - `position_open`
	/// - `position_closed`
	/// - `tilt_min`
	/// - `tilt_max`
	///
	/// The `entity_id` can be used to reference the entity’s attributes
	/// with the help of the [states][states] template function.
	///
	/// [template]: https://www.home-assistant.io/docs/configuration/templating/#processing-incoming-data
	/// [states]: https://www.home-assistant.io/docs/configuration/templating/#states
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub tilt_status_template: Option<Template<'a>>,

	/// The MQTT topic subscribed to receive tilt status update values.
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub tilt_status_topic: Option<Topic<'a>>,

	/// Defines a [template][template] that can be used to extract the
	/// payload for the `state_topic` topic.
	///
	/// [template]: https://www.home-assistant.io/docs/configuration/templating/#processing-incoming-data
	#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
	pub value_template: Option<Template<'a>>,
}

impl<'a> Validator for Cover<'a> {
	type Invalidity = CoverInvalidity;

	fn validate_value(
		&self,
		value: &Self,
		context: semval::context::Context<Self::Invalidity>,
	) -> semval::context::Context<Self::Invalidity> {
		// https://github.com/home-assistant/core/blob/6cfb40f080279a45c8e630419742cc2f3faedecb/homeassistant/components/mqtt/cover.py#L116
		context
			.invalidate_if(
				value.value_template.is_some() && value.state_topic.is_none(),
				CoverInvalidity::StateTopicMissing,
			)
			.invalidate_if(
				value.set_position_topic.is_some() && value.position_topic.is_none(),
				CoverInvalidity::PositionTopicMissing,
			)
			.invalidate_if(
				value.position_template.is_some() && value.position_topic.is_none(),
				CoverInvalidity::PositionTopicMissing,
			)
			.invalidate_if(
				value.set_position_template.is_some() && value.set_position_topic.is_none(),
				CoverInvalidity::SetPositionTopicMissing,
			)
			.invalidate_if(
				value.tilt_command_template.is_some() && value.tilt_command_topic.is_none(),
				CoverInvalidity::TiltCommandTopicMissing,
			)
			.invalidate_if(
				value.tilt_status_template.is_some() && value.tilt_status_topic.is_none(),
				CoverInvalidity::TiltStatusTopicMissing,
			)
	}
}
