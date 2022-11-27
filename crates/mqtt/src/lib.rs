mod availability;
mod client;
mod options;
mod tracking;

pub use options::HassMqttOptions;

#[repr(u8)]
pub enum MqttQosLevel {
	AtLeastOnce = 0,
	AtMostOnce = 1,
	ExactlyOnce = 2,
}

#[cfg(test)]
mod tests {
	#[test]
	fn it_works() {
		assert_eq!(2 + 2, 4);
	}
}
