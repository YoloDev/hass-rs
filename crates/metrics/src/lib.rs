use opentelemetry::metrics as otel;
use opentelemetry::Context as OtelContext;
use opentelemetry::{Key, KeyValue, Value};
use std::borrow::Cow;
use std::sync::Arc;

mod intern {
	use lasso::ThreadedRodeo;
	use once_cell::sync::OnceCell;
	use opentelemetry::Key;
	use std::borrow::Cow;

	static LABELS: OnceCell<ThreadedRodeo> = OnceCell::new();

	fn labels() -> &'static ThreadedRodeo {
		LABELS.get_or_init(ThreadedRodeo::default)
	}

	pub(super) fn get_or_intern(value: Cow<'static, str>) -> Key {
		Key::from_static_str(match value {
			Cow::Borrowed(value) => value,
			Cow::Owned(value) => {
				let labels = labels();
				let spur = labels.get_or_intern(value);
				labels.resolve(&spur)
			}
		})
	}
}

pub trait MetricField: Sized {
	fn into_value(self) -> Value;
}

impl MetricField for String {
	fn into_value(self) -> Value {
		Value::from(self)
	}
}

impl MetricField for &'static str {
	fn into_value(self) -> Value {
		Value::from(self)
	}
}

impl MetricField for Arc<str> {
	fn into_value(self) -> Value {
		Value::from(self)
	}
}

impl MetricField for Cow<'static, str> {
	fn into_value(self) -> Value {
		Value::from(self)
	}
}

impl MetricField for u32 {
	fn into_value(self) -> Value {
		Value::from(self as i64)
	}
}

impl MetricField for i64 {
	fn into_value(self) -> Value {
		Value::from(self)
	}
}

impl MetricField for f64 {
	fn into_value(self) -> Value {
		Value::from(self)
	}
}

impl MetricField for bool {
	fn into_value(self) -> Value {
		Value::from(self)
	}
}

impl MetricField for u16 {
	fn into_value(self) -> Value {
		Value::from(self as i64)
	}
}

pub trait MetricFields {
	const LENGTH: usize;
	type Init;
	type Keys;
	type Values;
	type KeyValues;

	fn intern(names: Self::Init) -> Self::Keys;
	fn zip(keys: &Self::Keys, values: Self::Values) -> Self::KeyValues;
}

impl MetricFields for () {
	const LENGTH: usize = 0;
	type Init = [Cow<'static, str>; 0];
	type Keys = [Key; 0];
	type Values = [Value; 0];
	type KeyValues = [KeyValue; 0];

	fn intern(_: Self::Init) -> Self::Keys {
		[]
	}

	fn zip(_: &Self::Keys, _: Self::Values) -> Self::KeyValues {
		[]
	}
}

impl<T1> MetricFields for (T1,)
where
	T1: MetricField,
{
	const LENGTH: usize = 1;
	type Init = [Cow<'static, str>; 1];
	type Keys = [Key; 1];
	type Values = [Value; 1];
	type KeyValues = [KeyValue; 1];

	fn intern(names: Self::Init) -> Self::Keys {
		let [n0] = names;
		[intern::get_or_intern(n0)]
	}

	fn zip(keys: &Self::Keys, values: Self::Values) -> Self::KeyValues {
		let [k0] = keys;
		let [v0] = values;
		[KeyValue::new(k0.clone(), v0)]
	}
}

impl<T1, T2> MetricFields for (T1, T2)
where
	T1: MetricField,
	T2: MetricField,
{
	const LENGTH: usize = 2;
	type Init = [Cow<'static, str>; 2];
	type Keys = [Key; 2];
	type Values = [Value; 2];
	type KeyValues = [KeyValue; 2];

	fn intern(names: Self::Init) -> Self::Keys {
		let [n0, n1] = names;
		[intern::get_or_intern(n0), intern::get_or_intern(n1)]
	}

	fn zip(keys: &Self::Keys, values: Self::Values) -> Self::KeyValues {
		let [k0, k1] = keys;
		let [v0, v1] = values;
		[KeyValue::new(k0.clone(), v0), KeyValue::new(k1.clone(), v1)]
	}
}

pub struct Counter<T: MetricFields> {
	inner: otel::Counter<u64>,
	field_names: <T as MetricFields>::Keys,
}

impl<T: MetricFields> Counter<T> {
	pub fn new(
		meter: &otel::Meter,
		name: impl Into<String>,
		description: impl Into<String>,
		field_names: <T as MetricFields>::Init,
	) -> Self {
		let field_names = T::intern(field_names);

		let inner = meter.u64_counter(name).with_description(description).init();
		Self { inner, field_names }
	}
}

impl Counter<()> {
	pub fn add_in_context(&self, cx: &OtelContext, value: u64) {
		self.inner.add(cx, value, &[])
	}

	pub fn add(&self, value: u64) {
		self.add_in_context(&OtelContext::current(), value)
	}
}

impl<T1> Counter<(T1,)>
where
	T1: MetricField,
{
	pub fn add_in_context(&self, cx: &OtelContext, value: u64, field1: T1) {
		let field_values = <(T1,) as MetricFields>::zip(&self.field_names, [field1.into_value()]);
		self.inner.add(cx, value, &field_values)
	}

	pub fn add(&self, value: u64, field1: T1) {
		self.add_in_context(&OtelContext::current(), value, field1)
	}
}

impl<T1, T2> Counter<(T1, T2)>
where
	T1: MetricField,
	T2: MetricField,
{
	pub fn add_in_context(&self, cx: &OtelContext, value: u64, field1: T1, field2: T2) {
		let field_values = <(T1, T2) as MetricFields>::zip(
			&self.field_names,
			[field1.into_value(), field2.into_value()],
		);
		self.inner.add(cx, value, &field_values)
	}

	pub fn add(&self, value: u64, field1: T1, field2: T2) {
		self.add_in_context(&OtelContext::current(), value, field1, field2)
	}
}

#[doc(hidden)]
#[macro_export]
macro_rules! counter {
	(@type ($($t:ty,)*)) => {
		$crate::Counter<($($t,)*)>
	};
}

#[macro_export]
macro_rules! metrics {
	(@meter_type Counter(
		$metric_name:literal,
		$metric_description:literal
		$(,
			$((
				$($metric_label:literal : $metric_label_ty:ty),*$(,)?
			)$(,)?)?
		)?
	)) => {
		$crate::counter!(@type ($($($($metric_label_ty,)*)?)?))
	};

	(@meter_init $meter:ident Counter(
		$metric_name:literal,
		$metric_description:literal
		$(,
			$((
				$($metric_label:literal : $metric_label_ty:ty),*$(,)?
			)$(,)?)?
		)?
	)) => {{
		$crate::Counter::new(
			&$meter,
			$metric_name,
			$metric_description,
			[
				$($($(::std::borrow::Cow::from($metric_label),)*)?)?
			],
		)
	}};

	($vis:vis struct $struct_name:ident {
		$(
			$fld_vis:vis $name:ident : $kind:ident $factory:tt
		),*$(,)?
	}) => {
		$vis struct $struct_name {
			$(
				$fld_vis $name: $crate::metrics!(@meter_type $kind $factory),
			)*
		}

		impl $struct_name {
			pub fn from_meter(meter: $crate::_export::otel::Meter) -> Self {
				Self {
					$(
						$name: $crate::metrics!(@meter_init meter $kind $factory),
					)*
				}
			}

			pub fn new() -> Self {
				let meter = $crate::_export::meter_with_version(
					env!("CARGO_PKG_NAME"),
					Some(env!("CARGO_PKG_VERSION")),
					None
				);

				Self::from_meter(meter)
			}

			pub fn global() -> &'static Self {
				static INSTANCE: $crate::_export::OnceCell<$struct_name> = $crate::_export::OnceCell::new();

				INSTANCE.get_or_init(Self::new)
			}
		}
	}
}

#[doc(hidden)]
pub mod _export {
	pub use once_cell::sync::OnceCell;
	pub use opentelemetry::global::meter_with_version;
	pub use opentelemetry::metrics as otel;
}
