use darling::{ast::Data, error::Accumulator, Error, FromDeriveInput, FromField, FromMeta};
use proc_macro2::Span;
use quote::format_ident;
use std::collections::BTreeMap;
use syn::{spanned::Spanned, Meta, MetaList, MetaNameValue, NestedMeta};

#[derive(FromDeriveInput, Debug)]
#[darling(attributes(entity, state), supports(struct_named), forward_attrs)]
pub(super) struct DocumentStructInput {
	pub ident: syn::Ident,
	pub vis: syn::Visibility,
	pub generics: syn::Generics,
	pub data: Data<(), DocumentFieldInput>,
	pub attrs: Vec<syn::Attribute>,
	#[darling(default)]
	pub extend_json: Option<AdditionalProps>,
	#[darling(default)]
	pub validate: Option<AdditionalInvalidities>,
}

#[derive(FromField, Debug)]
#[darling(attributes(entity, state), forward_attrs)]
pub(super) struct DocumentFieldInput {
	// guaranteed to never be `None` by `darling`
	pub ident: Option<syn::Ident>,
	pub ty: syn::Type,
	pub attrs: Vec<syn::Attribute>,
	pub validate: FieldValidation,
	pub builder: Builder,
	pub vis: syn::Visibility,
}

#[derive(Debug)]
pub(super) struct Builder {
	pub enabled: bool,
	pub rename: Option<syn::Ident>,
	span: Option<Span>,
}

impl Default for Builder {
	fn default() -> Self {
		Builder {
			enabled: true,
			rename: None,
			span: None,
		}
	}
}

impl Spanned for Builder {
	fn span(&self) -> Span {
		self.span.unwrap_or_else(Span::call_site)
	}
}

impl FromMeta for Builder {
	fn from_none() -> Option<Self> {
		Some(Builder::default())
	}

	fn from_meta(mi: &syn::Meta) -> darling::Result<Self> {
		match mi {
			syn::Meta::Path(_) => Ok(Builder {
				span: Some(mi.span()),
				..Default::default()
			}),

			syn::Meta::NameValue(MetaNameValue {
				lit: syn::Lit::Bool(b),
				..
			}) => Ok(Builder {
				enabled: b.value,
				span: Some(mi.span()),
				..Default::default()
			}),

			syn::Meta::NameValue(MetaNameValue {
				lit: syn::Lit::Str(s),
				..
			}) => Ok(Builder {
				enabled: true,
				rename: Some(format_ident!("{}", s.value(), span = s.span())),
				span: Some(mi.span()),
			}),

			_ => {
				// The implementation for () will produce an error for all non-path meta items;
				// call it to make sure the span behaviors and error messages are the same.
				Err(<()>::from_meta(mi).unwrap_err())
			}
		}
	}
}

#[derive(Debug)]
pub(super) enum FieldValidation {
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

impl Spanned for FieldValidation {
	fn span(&self) -> Span {
		match self {
			Self::None => Span::call_site(),
			Self::Default(span) => span.unwrap_or_else(Span::call_site),
			Self::With(span, _) => *span,
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
