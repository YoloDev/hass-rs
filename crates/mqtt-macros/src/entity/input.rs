use darling::{ast::Data, error::Accumulator, Error, FromDeriveInput, FromField, FromMeta};
use proc_macro2::Span;
use quote::quote;
use std::{collections::BTreeMap, mem};
use syn::{spanned::Spanned, FieldsNamed, Meta, MetaList, NestedMeta};

fn common_fields() -> Vec<EntityFieldInput> {
	let tokens = quote! {{
		/// A list of MQTT topics subscribed to receive availability (online/offline) updates.
		#[serde(borrow, default, skip_serializing_if = "<[crate::availability::Availability]>::is_empty")]
		#[entity(validate)]
		pub availability: ::std::borrow::Cow<'a, [crate::availability::Availability<'a>]>,

		/// When `availability` is configured, this controls the conditions needed
		/// to set the entity to `available`.
		#[serde(default, skip_serializing_if = "crate::availability::AvailabilityMode::is_default")]
		pub availability_mode: crate::availability::AvailabilityMode,

		/// Information about the device this entity is a part of to tie it into the device registry.
		/// Only works through MQTT discovery and when `unique_id` is set.
		/// At least one of identifiers or connections must be present to identify the device.
		#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
		#[entity(validate)]
		pub device: Option<crate::device::Device<'a>>,

		/// Flag which defines if the entity should be enabled when first added.
		/// Defaults to `true`.
		#[serde(default, skip_serializing_if = "Option::is_none")]
		pub enabled_by_default: Option<bool>,

		/// The encoding of the payloads received and published messages. Set to "" to disable decoding of incoming payload.
		/// Defaults to `"utf-8"`.
		#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
		pub encoding: Option<::std::borrow::Cow<'a, str>>,

		/// The [category] of the entity.
		///
		/// [category]: https://developers.home-assistant.io/docs/core/entity#generic-properties
		#[serde(default, skip_serializing_if = "crate::entity_category::EntityCategory::is_none")]
		pub entity_category: crate::entity_category::EntityCategory,

		/// [Icon][icon] for the entity.
		///
		/// [icon]: https://www.home-assistant.io/docs/configuration/customizing-devices/#icon
		#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
		#[entity(validate)]
		pub icon: Option<crate::icon::Icon<'a>>,

		/// Defines a [template][template] to extract the JSON dictionary from messages received
		/// on the `json_attributes_topic`.
		///
		/// [template]: https://www.home-assistant.io/docs/configuration/templating/#processing-incoming-data
		#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
		#[entity(validate)]
		pub json_attributes_template: Option<crate::template::Template<'a>>,

		/// The MQTT topic subscribed to receive a JSON dictionary payload and then set as entity
		/// attributes.
		///
		/// Implies `force_update` of the current state when a message is received on this topic.
		#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
		#[entity(validate)]
		pub json_attributes_topic: Option<crate::topic::Topic<'a>>,

		/// The name of the MQTT entity.
		#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
		#[entity(validate)]
		pub name: Option<crate::name::Name<'a>>,

		/// Used instead of `name` for automatic generation of `entity_id`.
		#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
		pub object_id: Option<::std::borrow::Cow<'a, str>>,

		/// The maximum QoS level of the state topic.
		#[serde(default, skip_serializing_if = "crate::qos::MqttQoS::is_default")]
		pub qos: crate::qos::MqttQoS,

		/// An ID that uniquely identifies this entity. If two entities have the same unique ID,
		/// Home Assistant will raise an exception.
		#[serde(borrow, default, skip_serializing_if = "Option::is_none")]
		#[entity(validate)]
		pub unique_id: Option<crate::unique_id::UniqueId<'a>>,
	}};

	let fields: FieldsNamed = syn::parse2(tokens).unwrap();
	fields
		.named
		.iter()
		.map(EntityFieldInput::from_field)
		.collect::<Result<Vec<_>, _>>()
		.unwrap()
}

#[derive(FromDeriveInput, Debug)]
#[darling(attributes(entity), supports(struct_named), forward_attrs)]
pub struct EntityStructInput {
	pub ident: syn::Ident,
	pub vis: syn::Visibility,
	pub generics: syn::Generics,
	pub data: Data<(), EntityFieldInput>,
	pub attrs: Vec<syn::Attribute>,
	#[darling(default)]
	pub extend_json: Option<AdditionalProps>,
	#[darling(default)]
	pub validate: Option<AdditionalInvalidities>,
}

#[derive(FromField, Debug)]
#[darling(attributes(entity), forward_attrs)]
pub struct EntityFieldInput {
	// guaranteed to never be `None` by `darling`
	pub ident: Option<syn::Ident>,
	pub ty: syn::Type,
	pub attrs: Vec<syn::Attribute>,
	pub validate: FieldValidation,
	pub vis: syn::Visibility,
}

#[derive(Debug)]
pub enum FieldValidation {
	None,
	Default(Option<Span>),
	With(Span, syn::Path),
}

impl FromMeta for FieldValidation {
	fn from_none() -> Option<Self> {
		Some(FieldValidation::None)
	}

	fn from_meta(mi: &syn::Meta) -> darling::Result<Self> {
		match mi {
			syn::Meta::Path(p) => Ok(Self::Default(Some(p.span()))),
			syn::Meta::NameValue(nv) => {
				let path = <syn::Path as FromMeta>::from_value(&nv.lit)?;
				Ok(Self::With(nv.span(), path))
			}
			_ => {
				// The implementation for () will produce an error for all non-path meta items;
				// call it to make sure the span behaviors and error messages are the same.
				Err(<()>::from_meta(mi).unwrap_err())
			}
		}
	}
}

#[derive(Debug, Default)]
pub struct AdditionalProps {
	values: BTreeMap<String, String>,
}

impl AdditionalProps {
	pub(crate) fn props(&self) -> impl Iterator<Item = (&str, &str)> {
		self.values.iter().map(|(k, v)| (&**k, &**v))
	}
}

impl FromMeta for AdditionalProps {
	fn from_meta(item: &Meta) -> darling::Result<Self> {
		let mut items = BTreeMap::new();

		let list = match item {
			Meta::List(list) => list,
			Meta::Path(_) => return Err(Error::unsupported_format("path").with_span(item)),
			Meta::NameValue(_) => return Err(Error::unsupported_format("name=value").with_span(item)),
		};

		let mut accumulator = Accumulator::default();
		for item in &list.nested {
			let (item_meta, name_value) = match item {
				NestedMeta::Lit(l) => {
					accumulator.push(Error::unsupported_format("literal").with_span(l));
					continue;
				}
				NestedMeta::Meta(Meta::Path(p)) => {
					accumulator.push(Error::unsupported_format("path").with_span(p));
					continue;
				}
				NestedMeta::Meta(Meta::List(l)) => {
					accumulator.push(Error::unsupported_format("list").with_span(l));
					continue;
				}
				NestedMeta::Meta(nv @ Meta::NameValue(value)) => (nv, value),
			};

			let segments = &name_value.path.segments;
			if segments.len() != 1 {
				accumulator.push(Error::unsupported_format("path").with_span(&name_value.path));
				continue;
			}

			let key = segments.first().unwrap().ident.to_string();
			let value = match <String as FromMeta>::from_meta(item_meta) {
				Ok(v) => v,
				Err(e) => {
					accumulator.push(e);
					continue;
				}
			};

			items.insert(key, value);
		}

		accumulator.finish_with(AdditionalProps { values: items })
	}
}

#[derive(Debug, Default)]
pub struct AdditionalInvalidities {
	values: Vec<syn::Variant>,
}

impl AdditionalInvalidities {
	pub(crate) fn variants(&self) -> impl Iterator<Item = &syn::Variant> {
		self.values.iter()
	}
}

impl AdditionalInvalidities {
	fn from_items(list: &MetaList) -> darling::Result<Self> {
		let mut accumulator = Accumulator::default();
		let mut values = Vec::new();
		for item in &list.nested {
			match item {
				NestedMeta::Lit(l) => {
					accumulator.push(Error::unsupported_format("literal").with_span(l));
					continue;
				}
				NestedMeta::Meta(Meta::NameValue(v)) => {
					accumulator.push(Error::unsupported_format("name=value").with_span(v));
					continue;
				}
				NestedMeta::Meta(Meta::Path(p)) => {
					let item = match Self::from_path_single(p) {
						Ok(v) => v,
						Err(e) => {
							accumulator.push(e);
							continue;
						}
					};
					values.push(item);
				}
				NestedMeta::Meta(Meta::List(l)) => {
					let item = match Self::from_list_single(l) {
						Ok(v) => v,
						Err(e) => {
							accumulator.push(e);
							continue;
						}
					};
					values.push(item);
				}
			}
		}

		accumulator.finish_with(Self { values })
	}

	fn from_path(path: &syn::Path) -> darling::Result<Self> {
		Self::from_path_single(path).map(|v| Self { values: vec![v] })
	}

	fn from_path_single(path: &syn::Path) -> darling::Result<syn::Variant> {
		let segments = &path.segments;
		if segments.len() != 1 {
			return Err(Error::unsupported_format("path").with_span(&path));
		}

		let key = &segments.first().unwrap().ident;
		let variant = syn::Variant {
			attrs: vec![],
			ident: key.clone(),
			fields: syn::Fields::Unit,
			discriminant: None,
		};

		Ok(variant)
	}

	fn from_list_single(list: &MetaList) -> darling::Result<syn::Variant> {
		let mut variant = Self::from_path_single(&list.path)?;
		let mut accumulator = Accumulator::default();
		let mut fields = Vec::new();
		for item in &list.nested {
			match item {
				NestedMeta::Lit(l) => {
					accumulator.push(Error::unsupported_format("literal").with_span(l));
					continue;
				}
				NestedMeta::Meta(Meta::List(l)) => {
					accumulator.push(Error::unsupported_format("list").with_span(l));
					continue;
				}
				NestedMeta::Meta(Meta::NameValue(v)) => {
					accumulator.push(Error::unsupported_format("name=value").with_span(v));
					continue;
				}
				NestedMeta::Meta(Meta::Path(p)) => {
					let field = syn::Field {
						attrs: vec![],
						vis: syn::Visibility::Inherited,
						ident: None,
						colon_token: None,
						ty: syn::Type::Path(syn::TypePath {
							qself: None,
							path: p.clone(),
						}),
					};
					fields.push(field);
				}
			}
		}

		variant.fields = syn::Fields::Unnamed(syn::FieldsUnnamed {
			paren_token: syn::token::Paren::default(),
			unnamed: fields.into_iter().collect(),
		});

		accumulator.finish_with(variant)
	}
}

impl FromMeta for AdditionalInvalidities {
	fn from_meta(item: &Meta) -> darling::Result<Self> {
		match item {
			Meta::List(list) => Self::from_items(list),
			Meta::Path(path) => Self::from_path(path),
			Meta::NameValue(_) => Err(Error::unsupported_format("name=value").with_span(item)),
		}
	}
}

trait VecExt {
	fn prepend(&mut self, items: Self);
}

impl<T> VecExt for Vec<T> {
	fn prepend(&mut self, mut items: Self) {
		mem::swap(self, &mut items);
		self.append(&mut items);
	}
}

pub fn from_derive_input(input: &syn::DeriveInput) -> darling::Result<EntityStructInput> {
	let mut result = EntityStructInput::from_derive_input(input)?;
	match &mut result.data {
		Data::Struct(data) => {
			data.fields.prepend(common_fields());
		}
		_ => unreachable!(),
	};

	Ok(result)
}
