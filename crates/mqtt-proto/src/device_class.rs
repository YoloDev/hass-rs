/// The type of data a sensor returns impacts how it is displayed in the frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "ser", derive(serde::Serialize))]
#[cfg_attr(feature = "de", derive(serde::Deserialize))]
pub enum DeviceClass {
	/// Generic sensor. This is the default and doesn’t need to be set.
	None,

	/// Air Quality Index.
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "aqi"))]
	AirQualityIndex,

	/// Percentage of battery that is left.
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "battery"))]
	Battery,

	/// Carbon Dioxide in CO2 (Smoke)
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "carbon_dioxide"))]
	CarbonDioxide,

	/// Carbon Monoxide in CO (Gas CNG/LPG)
	#[cfg_attr(
		any(feature = "ser", feature = "de"),
		serde(rename = "carbon_monoxide")
	)]
	CarbonMonoxide,

	/// Current in A.
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "current"))]
	Current,

	/// Date string (ISO 8601).
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "date"))]
	Date,

	/// Energy in Wh or kWh.
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "energy"))]
	Energy,

	/// Gasvolume in m³ or ft³.
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "gas"))]
	Gas,

	/// Percentage of humidity in the air.
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "humidity"))]
	Humidity,

	/// The current light level in lx or lm.
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "illuminance"))]
	Illuminance,

	/// The monetary value.
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "monetary"))]
	Monetary,

	/// Concentration of Nitrogen Dioxide in µg/m³
	#[cfg_attr(
		any(feature = "ser", feature = "de"),
		serde(rename = "nitrogen_dioxide")
	)]
	NitrogenDioxide,

	/// Concentration of Nitrogen Monoxide in µg/m³
	#[cfg_attr(
		any(feature = "ser", feature = "de"),
		serde(rename = "nitrogen_monoxide")
	)]
	NitrogenMonoxide,

	/// Concentration of Nitrous Oxide in µg/m³
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "nitrous_oxide"))]
	NitrousOxide,

	/// Concentration of Ozone in µg/m³
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "ozone"))]
	Ozone,

	/// Concentration of particulate matter less than 1 micrometer in µg/m³
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "pm1"))]
	Pm1,

	/// Concentration of particulate matter less than 10 micrometers in µg/m³
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "pm10"))]
	Pm10,

	/// Concentration of particulate matter less than 2.5 micrometers in µg/m³
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "pm25"))]
	Pm25,

	/// Power factor in %.
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "power_factor"))]
	PowerFactor,

	/// Power in W or kW.
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "power"))]
	Power,

	/// Pressure in hPa or mbar.
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "pressure"))]
	Pressure,

	/// Signal strength in dB or dBm.
	#[cfg_attr(
		any(feature = "ser", feature = "de"),
		serde(rename = "signal_strength")
	)]
	SignalStrength,

	/// Concentration of sulphur dioxide in µg/m³
	#[cfg_attr(
		any(feature = "ser", feature = "de"),
		serde(rename = "sulphur_dioxide")
	)]
	SulphurDioxide,

	/// Temperature in °C or °F.
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "temperature"))]
	Temperature,

	/// Datetime object or timestamp string (ISO 8601).
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "timestamp"))]
	Timestamp,

	/// Concentration of volatile organic compounds in µg/m³.
	#[cfg_attr(
		any(feature = "ser", feature = "de"),
		serde(rename = "volatile_organic_compounds")
	)]
	VolatileOrganicCompounds,

	/// Voltage in V.
	#[cfg_attr(any(feature = "ser", feature = "de"), serde(rename = "voltage"))]
	Voltage,
}

impl DeviceClass {
	#[inline]
	pub const fn is_none(&self) -> bool {
		matches!(self, Self::None)
	}
}

impl Default for DeviceClass {
	#[inline]
	fn default() -> Self {
		Self::None
	}
}
