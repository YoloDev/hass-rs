use darling::{ast::Data, util::Flag, FromDeriveInput, FromField};
use quote::quote;
use std::mem;
use syn::FieldsNamed;

fn common_fields() -> Vec<EntityFieldInput> {
  let tokens = quote! {{
    /// A list of MQTT topics subscribed to receive availability (online/offline) updates.
    #[serde(borrow, default, skip_serializing_if = "<[Availability]>::is_empty")]
    #[entity(validate)]
    availability: Cow<'a, [Availability<'a>]>,

    /// When `availability` is configured, this controls the conditions needed
    /// to set the entity to `available`.
    #[serde(default, skip_serializing_if = "AvailabilityMode::is_default")]
    availability_mode: AvailabilityMode,

    /// Information about the device this entity is a part of to tie it into the device registry.
    /// Only works through MQTT discovery and when `unique_id` is set.
    /// At least one of identifiers or connections must be present to identify the device.
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    #[entity(validate)]
    device: Option<Device<'a>>,

    /// Flag which defines if the entity should be enabled when first added.
    /// Defaults to `true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    enabled_by_default: Option<bool>,

    /// The encoding of the payloads received and published messages. Set to "" to disable decoding of incoming payload.
    /// Defaults to `"utf-8"`.
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    encoding: Option<Cow<'a, str>>,

    /// The [category] of the entity.
    ///
    /// [category]: https://developers.home-assistant.io/docs/core/entity#generic-properties
    #[serde(default, skip_serializing_if = "EntityCategory::is_none")]
    entity_category: EntityCategory,

    /// [Icon][icon] for the entity.
    ///
    /// [icon]: https://www.home-assistant.io/docs/configuration/customizing-devices/#icon
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    #[entity(validate)]
    icon: Option<Icon<'a>>,

    /// Defines a [template][template] to extract the JSON dictionary from messages received
    /// on the `json_attributes_topic`.
    ///
    /// [template]: https://www.home-assistant.io/docs/configuration/templating/#processing-incoming-data
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    #[entity(validate)]
    json_attributes_template: Option<Template<'a>>,

    /// The MQTT topic subscribed to receive a JSON dictionary payload and then set as entity
    /// attributes.
    ///
    /// Implies `force_update` of the current state when a message is received on this topic.
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    #[entity(validate)]
    json_attributes_topic: Option<Topic<'a>>,

    /// The name of the MQTT entity.
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    #[entity(validate)]
    name: Option<Name<'a>>,

    /// Used instead of `name` for automatic generation of `entity_id`.
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    object_id: Option<Cow<'a, str>>,

    /// The maximum QoS level of the state topic.
    #[serde(default, skip_serializing_if = "MqttQoS::is_default")]
    qos: MqttQoS,

    /// An ID that uniquely identifies this entity. If two entities have the same unique ID,
    /// Home Assistant will raise an exception.
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    #[entity(validate)]
    unique_id: Option<UniqueId<'a>>,
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
}

#[derive(FromField, Debug)]
#[darling(attributes(entity), forward_attrs)]
pub struct EntityFieldInput {
  // guaranteed to never be `None` by `darling`
  pub ident: Option<syn::Ident>,
  pub ty: syn::Type,
  pub attrs: Vec<syn::Attribute>,
  pub validate: Flag,
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
