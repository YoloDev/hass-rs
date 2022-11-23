use super::EntityStruct;
use crate::util::ModifyLifetimes;
use darling::ToTokens;
use proc_macro2::TokenStream;
use quote::quote;

struct InvalidityEnum<'a>(&'a EntityStruct);

impl<'a> ToTokens for InvalidityEnum<'a> {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let vis = &self.0.vis;
    let ident = &self.0.invalidity_ident;
    let variants = self.0.fields.iter().filter_map(|f| {
      f.validate.then(|| {
        let ty = f.ty.make_lifetimes_static();
        let variant_ident = &f.variant_ident;
        quote! {
          #variant_ident(<#ty as ::semval::Validate>::Invalidity)
        }
      })
    });

    tokens.extend(quote! {
      #[derive(Copy, Clone, Debug, Eq, PartialEq)]
      #vis enum #ident {
        #(#variants,)*
      }
    })
  }
}

pub(super) fn invalidity_enum(entity: &EntityStruct) -> impl ToTokens + '_ {
  InvalidityEnum(entity)
}
