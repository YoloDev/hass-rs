use semval::Validate;

#[cfg(feature = "ser")]
use semval::Validated;

pub trait Document: Sized + Validate {
	#[cfg(feature = "ser")]
	fn serialize_validated<S: serde::Serializer>(
		validated: Validated<&Self>,
		serializer: S,
	) -> Result<S::Ok, S::Error>;

	#[cfg(feature = "ser")]
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		use core::fmt;
		use semval::IntoValidated;

		struct DisplayDebug<T: fmt::Debug>(T);
		impl<T: fmt::Debug> fmt::Display for DisplayDebug<T> {
			fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
				<T as fmt::Debug>::fmt(&self.0, f)
			}
		}

		let validated = self
			.into_validated()
			.map_err(|e| serde::ser::Error::custom(DisplayDebug(e.1)))?;

		Self::serialize_validated(validated, serializer)
	}
}
