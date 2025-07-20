pub(crate) mod input;

mod builders;
mod ctor;
mod document;
mod invalidity;
mod serde;
mod validate;

use self::input::{AdditionalInvalidities, AdditionalProps};
use crate::args::Args;
use convert_case::{Case, Casing};
use darling::{error::Accumulator, usage::GenericsExt, Error, FromDeriveInput, Result};
use proc_macro2::Span;
use quote::{format_ident, ToTokens};

pub(crate) struct DocumentStruct {
	ident: syn::Ident,
	invalidity_ident: syn::Ident,
	vis: syn::Visibility,
	generics: syn::Generics,
	docs: Vec<syn::Attribute>,
	attrs: Vec<syn::Attribute>,
	fields: Vec<DocumentField>,
	additional_invalidities: Option<AdditionalInvalidities>,
	additional_props: Option<AdditionalProps>,
}

impl DocumentStruct {
	pub(crate) fn document_struct<'a>(&'a self, args: &'a Args) -> impl ToTokens + 'a {
		document::document_struct(self, args)
	}

	pub(crate) fn ctor(&self) -> impl ToTokens + '_ {
		ctor::ctor(self)
	}

	pub(crate) fn builders(&self) -> impl ToTokens + '_ {
		builders::builders(self)
	}

	pub(crate) fn serde(&self) -> impl ToTokens + '_ {
		serde::serde_impl(self)
	}

	pub(crate) fn invalidity_enum(&self) -> impl ToTokens + '_ {
		invalidity::invalidity_enum(self)
	}

	pub(crate) fn validate(&self) -> impl ToTokens + '_ {
		validate::validation(self)
	}
}

impl TryFrom<input::DocumentStructInput> for DocumentStruct {
	type Error = darling::Error;

	fn try_from(value: input::DocumentStructInput) -> Result<Self> {
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
						Error::custom("Documents must only have a single lifetime named 'a")
							.with_span(&lifetime.ident),
					);
				}
			}

			if !has_a {
				let error = Error::custom("Documents must have a lifetime 'a");
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
				accumulator.push(Error::custom("Documents must be structs").with_span(&value.ident));
				Vec::new()
			}
			Some(data) if !data.is_struct() => {
				accumulator.push(Error::custom("Documents must be structs").with_span(&value.ident));
				Vec::new()
			}
			Some(data) => {
				let mut fields = Vec::with_capacity(data.len());
				for field in data {
					if let Some(field) = accumulator.handle(DocumentField::try_from(field)) {
						fields.push(field);
					}
				}

				fields
			}
		};

		let (docs, attrs): (Vec<_>, Vec<_>) = value
			.attrs
			.into_iter()
			.partition(|attr| attr.path().is_ident("doc"));

		let invalidity_ident = format_ident!("{}Invalidity", &value.ident, span = Span::call_site());

		accumulator.finish_with(Self {
			ident: value.ident,
			invalidity_ident,
			vis: value.vis,
			generics: value.generics,
			fields,
			docs,
			attrs,
			additional_invalidities: value.validate,
			additional_props: value.extend_json,
		})
	}
}

pub(crate) struct DocumentField {
	ident: syn::Ident,
	variant_ident: syn::Ident,
	ty: syn::Type,
	docs: Vec<syn::Attribute>,
	serde: Vec<syn::Attribute>,
	attrs: Vec<syn::Attribute>,
	validate: FieldValidation,
	builder: Builder,
	required: bool,
}

enum FieldValidation {
	None,
	Default(Option<Span>),
	With(Span, syn::Path),
}
impl FieldValidation {
	fn then<R>(&self, f: impl FnOnce(&Span, Option<&syn::Path>) -> R) -> Option<R> {
		match self {
			Self::None => None,
			Self::Default(span) => {
				let span = match span.as_ref() {
					Some(span) => span,
					None => &Span::call_site(),
				};
				Some(f(span, None))
			}
			Self::With(span, path) => Some(f(span, Some(path))),
		}
	}
}

impl From<input::FieldValidation> for FieldValidation {
	fn from(value: input::FieldValidation) -> Self {
		match value {
			input::FieldValidation::None => Self::None,
			input::FieldValidation::Default(span) => Self::Default(span),
			input::FieldValidation::With(span, path) => Self::With(span, path),
		}
	}
}

#[derive(Debug)]
struct Builder {
	pub enabled: bool,
	pub rename: Option<syn::Ident>,
	pub span: Span,
}

impl From<input::Builder> for Builder {
	fn from(value: input::Builder) -> Self {
		Builder {
			span: value.span(),
			enabled: value.enabled,
			rename: value.rename,
		}
	}
}

impl TryFrom<input::DocumentFieldInput> for DocumentField {
	type Error = darling::Error;

	fn try_from(value: input::DocumentFieldInput) -> Result<Self> {
		let mut accumulator = Accumulator::default();
		let ident = match value.ident {
			Some(ident) => ident,
			None => {
				accumulator.push(Error::custom("Document fields must be named"));
				syn::Ident::new("unknown", proc_macro2::Span::call_site())
			}
		};

		if !matches!(value.vis, syn::Visibility::Public(_)) {
			accumulator.push(Error::custom("Document fields must be public").with_span(&ident));
		}

		let ty = value.ty;
		let mut docs = Vec::with_capacity(value.attrs.len());
		let mut serde = Vec::with_capacity(2);
		let mut attrs = Vec::with_capacity(value.attrs.len());
		for attr in value.attrs {
			if attr.path().is_ident("doc") {
				docs.push(attr);
			} else if attr.path().is_ident("serde") {
				serde.push(attr);
			} else {
				attrs.push(attr);
			}
		}
		let validate = value.validate.into();
		let builder = value.builder.into();
		let has_default = serde
			.iter()
			.filter_map(|attr| match &attr.meta {
				syn::Meta::List(list) => Some(list),
				_ => None,
			})
			.any(|list| list.tokens.to_string().contains("default"));
		let required = !has_default;

		let variant_ident = format_ident!(
			"{}",
			ident
				.to_string()
				.from_case(Case::Snake)
				.to_case(Case::Pascal),
			span = Span::call_site(),
		);

		accumulator.finish_with(Self {
			ident,
			variant_ident,
			ty,
			attrs,
			docs,
			serde,
			validate,
			builder,
			required,
		})
	}
}

impl FromDeriveInput for DocumentStruct {
	fn from_derive_input(input: &syn::DeriveInput) -> darling::Result<Self> {
		let result = input::DocumentStructInput::from_derive_input(input)?;

		result.try_into()
	}
}
