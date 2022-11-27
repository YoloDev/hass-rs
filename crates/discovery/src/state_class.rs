use serde::{Deserialize, Serialize};

/// Classification of a non-primary entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum StateClass {
	/// Default - unspecified state class.
	None,

	/// The state represents _a measurement in present time_, not a historical
	/// aggregation such as statistics or a prediction of the future. Examples
	/// of what should be classified `measurement` are: current temperature,
	/// humidify or electric power. Examples of what should not be classified
	/// as `measurement`: Forecasted temperature for tomorrow, yesterday's
	/// energy consumption or anything else that doesn't include the _current_
	/// measurement. For supported sensors, statistics of hourly min, max and
	/// average sensor readings is updated every 5 minutes.
	#[serde(rename = "measurement")]
	Measurement,

	/// The state represents a total amount that can both increase and decrease,
	/// e.g. a net energy meter. Statistics of the accumulated growth or decline
	/// of the sensor's value since it was first added is updated every 5 minutes.
	/// This state class should not be used for sensors where the absolute value
	/// is interesting instead of the accumulated growth or decline, for example
	/// remaining battery capacity or CPU load; in such cases state class
	/// `measurement` should be used instead.
	#[serde(rename = "total")]
	Total,

	/// Similar to [Total][StateClass::Total], with the restriction that the state represents a
	/// monotonically increasing positive total, e.g. a daily amount of consumed
	/// gas, weekly water consumption or lifetime energy consumption. Statistics
	/// of the accumulated growth of the sensor's value since it was first added
	/// is updated every 5 minutes.
	#[serde(rename = "total_increasing")]
	TotalIncreasing,
}

impl StateClass {
	#[inline]
	pub const fn is_none(&self) -> bool {
		matches!(self, Self::None)
	}
}

impl Default for StateClass {
	#[inline]
	fn default() -> Self {
		Self::None
	}
}
