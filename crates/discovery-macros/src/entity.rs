mod entity_struct;
mod input;
mod invalidity;
mod validate;

use convert_case::{Case, Casing};
use darling::{error::Accumulator, usage::GenericsExt, Error, Result};
use proc_macro2::TokenStream;
use quote::{format_ident, ToTokens};
use syn::parse2;

struct EntityStruct {
  ident: syn::Ident,
  invalidity_ident: syn::Ident,
  vis: syn::Visibility,
  generics: syn::Generics,
  docs: Vec<syn::Attribute>,
  attrs: Vec<syn::Attribute>,
  fields: Vec<EntityField>,
}

impl EntityStruct {
  fn entity_struct(&self) -> impl ToTokens + '_ {
    entity_struct::entity_struct(self)
  }

  fn invalidity_enum(&self) -> impl ToTokens + '_ {
    invalidity::invalidity_enum(self)
  }

  fn validate(&self) -> impl ToTokens + '_ {
    validate::validation(self)
  }
}

impl TryFrom<input::EntityStructInput> for EntityStruct {
  type Error = darling::Error;

  fn try_from(value: input::EntityStructInput) -> Result<Self> {
    let mut accumulator = Accumulator::default();
    {
      let lifetimes = value.generics.declared_lifetimes();
      let mut has_a = false;
      let mut first = true;
      for lifetime in lifetimes {
        if first {
          first = false;
        }

        if lifetime.ident == "a" {
          has_a = true;
        } else {
          accumulator.push(
            Error::custom("Entities must only have a single lifetime named 'a")
              .with_span(&lifetime.ident),
          );
        }
      }

      if !has_a {
        let error = Error::custom("Entities must have a lifetime 'a");
        let error = if first {
          error.with_span(&value.ident)
        } else {
          error.with_span(&value.generics)
        };
        accumulator.push(error);
      }
    }

    let fields = match value.data.take_struct() {
      None => {
        accumulator.push(Error::custom("Entities must be structs").with_span(&value.ident));
        Vec::new()
      }
      Some(data) if !data.is_struct() => {
        accumulator.push(Error::custom("Entities must be structs").with_span(&value.ident));
        Vec::new()
      }
      Some(data) => {
        let mut fields = Vec::with_capacity(data.len());
        for field in data {
          if let Some(field) = accumulator.handle(EntityField::try_from(field)) {
            fields.push(field);
          }
        }

        fields
      }
    };

    let (docs, attrs): (Vec<_>, Vec<_>) = value
      .attrs
      .into_iter()
      .partition(|attr| attr.path.is_ident("doc"));

    let invalidity_ident = format_ident!("{}Invalidity", &value.ident);

    accumulator.finish_with(Self {
      ident: value.ident,
      invalidity_ident,
      vis: value.vis,
      generics: value.generics,
      fields,
      docs,
      attrs,
    })
  }
}

impl ToTokens for EntityStruct {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    self.entity_struct().to_tokens(tokens);
    self.invalidity_enum().to_tokens(tokens);
    self.validate().to_tokens(tokens);
  }
}

struct EntityField {
  ident: syn::Ident,
  variant_ident: syn::Ident,
  ty: syn::Type,
  docs: Vec<syn::Attribute>,
  attrs: Vec<syn::Attribute>,
  validate: bool,
  required: bool,
}

impl TryFrom<input::EntityFieldInput> for EntityField {
  type Error = darling::Error;

  fn try_from(value: input::EntityFieldInput) -> Result<Self> {
    let accumulator = Accumulator::default();
    let ident = match value.ident {
      Some(ident) => ident,
      None => syn::Ident::new("unknown", proc_macro2::Span::call_site()),
    };

    let ty = value.ty;
    let (docs, attrs): (Vec<_>, Vec<_>) = value
      .attrs
      .into_iter()
      .partition(|attr| attr.path.is_ident("doc"));
    let validate = value.validate.is_present();
    let has_default = attrs
      .iter()
      .any(|attr| attr.path.is_ident("serde") && attr.tokens.to_string().contains("default"));
    let required = !has_default;

    let variant_ident = format_ident!(
      "{}",
      ident
        .to_string()
        .from_case(Case::Snake)
        .to_case(Case::Pascal),
      span = ident.span()
    );

    accumulator.finish_with(Self {
      ident,
      variant_ident,
      ty,
      attrs,
      docs,
      validate,
      required,
    })
  }
}

// pub fn derive(input: syn::DeriveInput) -> Result<proc_macro2::TokenStream, darling::Error> {
//   todo!()
// }

pub fn create(input: TokenStream) -> Result<TokenStream> {
  let parsed: syn::DeriveInput = parse2(input)?;
  let entity = input::from_derive_input(&parsed)?;
  EntityStruct::try_from(entity).map(ToTokens::into_token_stream)
}
