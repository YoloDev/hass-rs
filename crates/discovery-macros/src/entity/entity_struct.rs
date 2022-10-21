use crate::util::ModifyLifetimes;

use super::EntityStruct;
use darling::ToTokens;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Path, PathArguments, Type};

struct BuilderStruct<'a>(&'a EntityStruct);

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

fn as_path(t: &Type) -> Option<&Path> {
  match t {
    Type::Path(p) => Some(&p.path),
    _ => None,
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

fn as_cow(p: &Path) -> Option<&Type> {
  match_path(p, &["std", "borrow", "Cow"]).and_then(|args| {
    if let PathArguments::AngleBracketed(args) = args {
      if args.args.len() == 1 {
        if let syn::GenericArgument::Type(t) = &args.args[0] {
          return Some(t);
        }
      } else if args.args.len() == 2 {
        if let syn::GenericArgument::Type(t) = &args.args[1] {
          return Some(t);
        }
      }
    }
    None
  })
}

impl<'a> ToTokens for BuilderStruct<'a> {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let vis = &self.0.vis;
    let generics = &self.0.generics;
    let ident = &self.0.ident;
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
        #ident: #ty
      }
    });

    let accessors = self.0.fields.iter().map(|f| {
      let ident = &f.ident;
      let docs = &f.docs;
      let (ret, ret_ty) = match &f.ty {
        syn::Type::Path(p) => {
          if let Some(inner) = as_option(&p.path) {
            if let Some(inner) = as_path(inner).and_then(as_cow) {
              (
                quote! {::std::option::Option::as_deref(&self.#ident)},
                quote! {::std::option::Option<&'_ #inner>},
              )
            } else {
              (
                quote! {::std::option::Option::as_ref(&self.#ident)},
                quote! {::std::option::Option<&'_ #inner>},
              )
            }
          } else if let Some(inner) = as_cow(&p.path) {
            (
              quote! {::std::ops::Deref::deref(&self.#ident)},
              quote! {&'_ #inner},
            )
          } else {
            (quote! {&self.#ident}, quote! {&'_ #p})
          }
        }
        // TODO: deal with?
        _ => panic!("type should be a path"),
      };

      quote! {
        #(#docs)*
        pub fn #ident(&self) -> #ret_ty {
          #ret
        }
      }
    });

    let builder_ident_string = format!("{}", self.0.builder_ident);

    tokens.extend(quote! {
      #(#docs)*
      #(#attrs)*
      #[derive(Debug, Clone, PartialEq, Eq, ::serde::Serialize, ::serde::Deserialize)]
      #[serde(try_from = #builder_ident_string)]
      #vis struct #ident #generics {
        #(#fields,)*
      }

      impl #generics #ident #generics {
        #(#accessors)*
      }
    });
  }
}

pub(super) fn entity_struct(entity: &EntityStruct) -> impl ToTokens + '_ {
  BuilderStruct(entity)
}
