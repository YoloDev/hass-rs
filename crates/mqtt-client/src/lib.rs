mod availability;
mod client;
mod entity;
mod options;
mod router;
mod topics;
mod tracking;

pub mod provider;

pub use options::HassMqttOptions;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum MqttQosLevel {
	AtLeastOnce = 0,
	AtMostOnce = 1,
	ExactlyOnce = 2,
}

impl From<MqttQosLevel> for u8 {
	fn from(qos: MqttQosLevel) -> Self {
		qos as u8
	}
}

impl From<MqttQosLevel> for i32 {
	fn from(qos: MqttQosLevel) -> Self {
		qos as i32
	}
}
