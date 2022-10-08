use super::EntityStruct;
use darling::ToTokens;
use proc_macro2::TokenStream;
use quote::quote;

struct BuilderStruct<'a>(&'a EntityStruct);

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

    let builder_ident_string = format!("{}", self.0.builder_ident);

    tokens.extend(quote! {
      #(#docs)*
      #(#attrs)*
      #[derive(Debug, Clone, PartialEq, Eq, ::serde::Serialize, ::serde::Deserialize)]
      #[serde(try_from = #builder_ident_string)]
      #vis struct #ident #generics {
        #(#fields,)*
      }
    });
  }
}

pub(super) fn entity_struct(entity: &EntityStruct) -> impl ToTokens + '_ {
  BuilderStruct(entity)
}
