use crate::util::ModifyLifetimes;

use super::EntityStruct;
use darling::ToTokens;
use itertools::MultiUnzip;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{Path, PathArguments, Type};

struct DocumentStruct<'a>(&'a EntityStruct);

fn match_path<'a>(path: &'a Path, segments: &[&str]) -> Option<&'a PathArguments> {
  if path.segments.len() == 1 && path.segments[0].ident == segments.last().unwrap() {
    Some(&path.segments[0].arguments)
  } else if path.leading_colon.is_some()
    && path.segments.len() == segments.len()
    && path
      .segments
      .iter()
      .zip(segments)
      .all(|(a, b)| a.ident == b)
  {
    Some(&path.segments.last().unwrap().arguments)
  } else {
    None
  }
}

fn as_option(p: &Path) -> Option<&Type> {
  match_path(p, &["std", "option", "Option"]).and_then(|args| {
    if let PathArguments::AngleBracketed(args) = args {
      if args.args.len() == 1 {
        if let syn::GenericArgument::Type(t) = &args.args[0] {
          return Some(t);
        }
      }
    }
    None
  })
}

impl<'a> ToTokens for DocumentStruct<'a> {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let vis = &self.0.vis;
    let generics = &self.0.generics;
    let ident = &self.0.ident;
    let proxy_ident = format_ident!("{}Proxy", &self.0.ident, span = Span::call_site());
    let docs = &self.0.docs;
    let attrs = &self.0.attrs;
    let fields = self.0.fields.iter().map(|f| {
      let ident = &f.ident;
      let docs = &f.docs;
      let attrs = &f.attrs;
      let ty = &f.ty;
      quote! {
        #(#docs)*
        #(#attrs)*
        pub #ident: #ty
      }
    });

    let (ctor_args, ctor_fields): (Vec<_>, Vec<_>) = self
      .0
      .fields
      .iter()
      .map(|f| {
        let ident = format_ident!("{}", &f.ident, span = Span::call_site());
        let ty = &f.ty;
        let required = &f.required;
        let (field, arg) = if *required {
          (
            quote! { #ident: ::std::convert::Into::into( #ident ) },
            Some(quote! { #ident: impl ::std::convert::Into::< #ty > }),
          )
        } else {
          (quote! { #ident: ::std::default::Default::default() }, None)
        };
        (arg, field)
      })
      .unzip();
    let ctor_args = ctor_args.into_iter().flatten();

    let mut proxy_generics = generics.clone();
    proxy_generics.params.clear();
    proxy_generics.params.push(syn::parse_quote!('b));
    proxy_generics.params.push(syn::parse_quote!('p));

    let proxy_inner_lifetime = proxy_generics.lifetimes().next().unwrap();
    let proxy_outer_lifetime = proxy_generics.lifetimes().last().unwrap();

    let proxy_fields = self.0.fields.iter().map(|f| {
      let ident = format_ident!("{}", &f.ident, span = Span::call_site());
      let attrs = &f.attrs;
      let ty = f.ty.make_lifetimes(&proxy_inner_lifetime.lifetime);
      quote! {
        #(#attrs)*
        #ident: & #proxy_outer_lifetime #ty
      }
    });

    let (ser_fns, additional_proxy_fields, additional_proxy_assigns) = match self
      .0
      .additional_props
      .as_ref()
    {
      None => (quote! {}, quote! {}, quote! {}),
      Some(v) => {
        let (fns, flds, assigns): (Vec<_>, Vec<_>, Vec<_>) = v.props().enumerate().map(|(idx, (name, value))| {
          let ident = format_ident!("__const_field_{}", idx);
          let ser_ident = format_ident!("__serialize_field_{}", idx);
          let ser_ident_str = ser_ident.to_string();
          let ser_fn = quote! {
            fn #ser_ident<S: ::serde::Serializer>(_: &(), s: S) -> ::std::result::Result<S::Ok, S::Error> {
              s.serialize_str(#value)
            }
          };
          let fld = quote! {
            #[serde(skip_deserializing, serialize_with = #ser_ident_str, rename = #name)]
            #ident: ()
          };
          let assign = quote!{ #ident: () };

          (ser_fn, fld, assign)
        }).multiunzip();

        (
          quote! { #(#fns)* },
          quote! { #(#flds,)* },
          quote! { #(#assigns,)* },
        )
      }
    };

    let proxy_assign = self.0.fields.iter().map(|f| {
      let ident = format_ident!("{}", &f.ident, span = Span::call_site());
      quote! {
        #ident: &doc.#ident
      }
    });

    let builders = self.0.fields.iter().map(|f| {
      let ident = format_ident!("{}", &f.ident, span = Span::call_site());
      let docs = &f.docs;
      let ty = &f.ty;
      match ty {
        syn::Type::Path(p) => {
          if let Some(inner) = as_option(&p.path) {
            let unset_ident = format_ident!("unset_{}", ident);
            quote! {
              #(#docs)*
              pub fn #ident(mut self, #ident: impl ::std::convert::Into< #inner >) -> Self {
                self.#ident = Some(#ident.into());
                self
              }

              #(#docs)*
              pub fn #unset_ident(&mut self) -> &mut Self {
                self.#ident = None;
                self
              }
            }
          } else {
            quote! {
              #(#docs)*
              pub fn #ident(mut self, #ident: impl ::std::convert::Into< #ty >) -> Self {
                self.#ident = #ident.into();
                self
              }
            }
          }
        }
        // TODO: deal with?
        _ => panic!("type should be a path"),
      }
    });

    tokens.extend(quote! {
      #(#docs)*
      #(#attrs)*
      #[derive(Debug, Clone, PartialEq, Eq, ::serde::Deserialize)]
      #vis struct #ident #generics {
        #(#fields,)*
      }

      impl #generics #ident #generics {
        pub fn new(#(#ctor_args,)* ) -> Self {
          Self {
            #(#ctor_fields,)*
          }
        }

        #(#builders)*
      }

      impl #generics crate::Document for #ident #generics {
        fn serialize_validated<S>(validated: ::semval::Validated::<& #ident #generics>, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
        where
          S: ::serde::Serializer,
        {
          #ser_fns

          #[derive(::serde::Serialize)]
          struct #proxy_ident #proxy_generics {
            #(#proxy_fields,)*
            #additional_proxy_fields
          }

          let doc = *validated;
          let proxy = #proxy_ident {
            #(#proxy_assign,)*
            #additional_proxy_assigns
          };

          <#proxy_ident as ::serde::Serialize>::serialize(
            &proxy,
            serializer,
          )
        }
      }

      impl #generics ::serde::Serialize for #ident #generics {
        #[inline]
        fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
        where
          S: ::serde::Serializer,
        {
          crate::Document::serialize(self, serializer)
        }
      }
    });
  }
}

pub(super) fn entity_struct(entity: &EntityStruct) -> impl ToTokens + '_ {
  DocumentStruct(entity)
}