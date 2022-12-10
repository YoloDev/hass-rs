use crate::{args::Args, json_doc::DocumentStruct, util::Prepend};
use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse2, FieldsNamed};

fn common_fields() -> FieldsNamed {
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
}

struct EntityStruct(DocumentStruct);

impl FromDeriveInput for EntityStruct {
	fn from_derive_input(input: &syn::DeriveInput) -> darling::Result<Self> {
		let mut input = input.clone();

		if let syn::Data::Struct(syn::DataStruct {
			fields: syn::Fields::Named(f),
			..
		}) = &mut input.data
		{
			f.prepend(common_fields());
		}

		DocumentStruct::from_derive_input(&input).map(Self)
	}
}

impl EntityStruct {
	fn into_token_stream(self, args: Args) -> TokenStream {
		let mut tokens = TokenStream::new();
		self.0.document_struct(&args).to_tokens(&mut tokens);
		self.0.ctor().to_tokens(&mut tokens);
		self.0.builders().to_tokens(&mut tokens);
		self.0.invalidity_enum().to_tokens(&mut tokens);
		self.0.validate().to_tokens(&mut tokens);
		self.0.serde().to_tokens(&mut tokens);
		tokens
	}
}

pub fn create(input: TokenStream, args: Args) -> darling::Result<TokenStream> {
	let parsed: syn::DeriveInput = parse2(input)?;
	let doc = EntityStruct::from_derive_input(&parsed)?;
	Ok(doc.into_token_stream(args))
}
