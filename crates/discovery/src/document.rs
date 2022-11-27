use semval::{IntoValidated, Validate, Validated};

pub trait Document: Sized + Validate {
	fn serialize_validated<S: serde::Serializer>(
		validated: Validated<&Self>,
		serializer: S,
	) -> Result<S::Ok, S::Error>;

	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		let validated = self
			.into_validated()
			.map_err(|e| serde::ser::Error::custom(format!("{:?}", e.1)))?;

		Self::serialize_validated(validated, serializer)
	}
}
