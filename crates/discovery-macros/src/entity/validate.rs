use super::EntityStruct;
use darling::ToTokens;
use proc_macro2::TokenStream;
use quote::quote;

struct ValidationImpl<'a>(&'a EntityStruct);

impl<'a> ToTokens for ValidationImpl<'a> {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let generics = &self.0.generics;
    let ident = &self.0.ident;
    let invalidity_ident = &self.0.invalidity_ident;

    let fields_validation = self.0.fields.iter().filter(|f| f.validate).map(|f| {
      let ident = &f.ident;
      let variant_ident = &f.variant_ident;
      quote! { .validate_with(&self.#ident, #invalidity_ident::#variant_ident) }
    });

    tokens.extend(quote! {
      impl #generics ::semval::Validate for #ident #generics {
        type Invalidity = #invalidity_ident;

        fn validate(&self) -> ::semval::ValidationResult<Self::Invalidity> {
          ::semval::context::Context::new()
            #(#fields_validation)*
            .into_result()
        }
      }
    });
  }
}

pub(super) fn validation(entity: &EntityStruct) -> impl ToTokens + '_ {
  ValidationImpl(entity)
}
