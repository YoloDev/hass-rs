use core::fmt;
use semval::{Invalidity, Validate};

#[cfg(feature = "spantrace")]
use tracing_error::SpanTrace;

#[cfg(feature = "backtrace")]
use std::backtrace::Backtrace;

#[cfg(provide_any)]
use std::any::{Demand, Provider};

use crate::HassItems;

#[derive(Debug)]
pub struct ValidationError<I: Invalidity + Send + Sync> {
	invalidity: I,
	#[cfg(feature = "backtrace")]
	backtrace: Backtrace,
	#[cfg(feature = "spantrace")]
	spantrace: SpanTrace,
}

impl<I: Invalidity + Send + Sync> ValidationError<I> {
	#[inline]
	pub fn new(invalidity: I) -> Self {
		Self {
			invalidity,
			#[cfg(feature = "backtrace")]
			backtrace: Backtrace::capture(),
			#[cfg(feature = "spantrace")]
			spantrace: SpanTrace::capture(),
		}
	}

	pub fn invalidity(&self) -> &I {
		&self.invalidity
	}

	pub fn into_invalidity(self) -> I {
		self.invalidity
	}

	#[cfg(feature = "backtrace")]
	#[cfg_attr(doc_cfg, doc(cfg(feature = "backtrace")))]
	pub fn backtrace(&self) -> &Backtrace {
		&self.backtrace
	}

	#[cfg(feature = "spantrace")]
	#[cfg_attr(doc_cfg, doc(cfg(feature = "spantrace")))]
	pub fn spantrace(&self) -> &SpanTrace {
		&self.spantrace
	}
}

impl<I: Invalidity + Send + Sync> From<I> for ValidationError<I> {
	fn from(invalidity: I) -> Self {
		Self::new(invalidity)
	}
}

impl<I: Invalidity + Send + Sync> fmt::Display for ValidationError<I> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "validation error: {:?}", &self.invalidity)
	}
}

#[cfg(provide_any)]
impl<I: Invalidity + Send + Sync> Provider for ValidationError<I> {
	fn provide<'a>(&'a self, demand: &mut Demand<'a>) {
		demand.provide_ref(&self.invalidity);
		#[cfg(feature = "backtrace")]
		demand.provide_ref(&self.backtrace);
		#[cfg(feature = "spantrace")]
		demand.provide_ref(&self.spantrace);
	}
}

pub trait Validator<T = Self> {
	type Invalidity: Invalidity;

	fn validate_value(
		&self,
		value: &T,
		context: semval::context::Context<Self::Invalidity>,
	) -> semval::context::Context<Self::Invalidity>;
}

pub(crate) trait ValidateContextExt {
	type Invalidity: Invalidity;

	/// Validate the target and merge the mapped result into this context if the target is not `None`.
	fn validate_with_opt<F, U>(self, target: &Option<impl Validate<Invalidity = U>>, map: F) -> Self
	where
		F: Fn(U) -> Self::Invalidity,
		U: Invalidity;

	/// Validate all items in an iterator.
	fn validate_iter<'a, F, U, I, II: 'a>(self, target: I, map: F) -> Self
	where
		F: Fn(usize, U) -> Self::Invalidity,
		U: Invalidity,
		I: IntoIterator<Item = &'a II>,
		II: Validate<Invalidity = U>;

	fn validate_using<U, T>(self, validator: &impl Validator<T, Invalidity = U>, value: &T) -> Self
	where
		U: Invalidity + Into<Self::Invalidity>;

	fn validate_using_with<U, T>(
		self,
		validator: &impl Validator<T, Invalidity = U>,
		value: &T,
		map: impl Fn(U) -> Self::Invalidity,
	) -> Self
	where
		U: Invalidity;
}

impl<V: Invalidity> ValidateContextExt for semval::context::Context<V> {
	type Invalidity = V;

	#[inline]
	fn validate_with_opt<F, U>(self, target: &Option<impl Validate<Invalidity = U>>, map: F) -> Self
	where
		F: Fn(U) -> Self::Invalidity,
		U: Invalidity,
	{
		match target {
			Some(v) => self.validate_with(v, map),
			None => self,
		}
	}

	fn validate_iter<'a, F, U, I, II: 'a>(self, target: I, map: F) -> Self
	where
		F: Fn(usize, U) -> Self::Invalidity,
		U: Invalidity,
		I: IntoIterator<Item = &'a II>,
		II: Validate<Invalidity = U>,
	{
		let mut ret = self;

		for (index, item) in target.into_iter().enumerate() {
			ret = ret.validate_with(item, |v| map(index, v));
		}

		ret
	}

	#[inline]
	fn validate_using<U, T>(self, validator: &impl Validator<T, Invalidity = U>, value: &T) -> Self
	where
		U: Invalidity + Into<Self::Invalidity>,
	{
		self.validate(&Using(validator, value))
	}

	#[inline]
	fn validate_using_with<U, T>(
		self,
		validator: &impl Validator<T, Invalidity = U>,
		value: &T,
		map: impl Fn(U) -> Self::Invalidity,
	) -> Self
	where
		U: Invalidity,
	{
		self.validate_with(&Using(validator, value), map)
	}
}

struct Using<'a, T, U>(&'a U, &'a T)
where
	U: Validator<T>;

impl<'a, T, U> Validate for Using<'a, T, U>
where
	U: Validator<T>,
{
	type Invalidity = U::Invalidity;

	fn validate(&self) -> semval::ValidationResult<Self::Invalidity> {
		self
			.0
			.validate_value(self.1, semval::context::Context::new())
			.into_result()
	}
}

impl<'a, T: Validate> Validate for HassItems<'a, T> {
	type Invalidity = <[T] as Validate>::Invalidity;

	#[inline]
	fn validate(&self) -> semval::ValidationResult<Self::Invalidity> {
		self.as_ref().validate()
	}
}
