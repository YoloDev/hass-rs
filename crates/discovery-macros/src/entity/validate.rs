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
    // let (fields, fields_init, validated_inits): (Vec<_>, Vec<_>, Vec<_>) = self
    //   .0
    //   .fields
    //   .iter()
    //   .map(|f| {
    //     let ident = &f.ident;
    //     let docs = &f.docs;
    //     let serde_attrs = f.attrs.iter().filter(|a| a.path.is_ident("serde"));
    //     let ty = &f.ty;
    //     let ty = if f.required {
    //       quote! {::std::option::Option<#ty>}
    //     } else {
    //       quote! {#ty}
    //     };
    //     let init = if f.required {
    //       quote! { Some(self.#ident) }
    //     } else {
    //       quote! { self.#ident }
    //     };
    //     let validated_init = if f.required {
    //       quote! { self.#ident.unwrap() }
    //     } else {
    //       quote! { self.#ident }
    //     };

    //     let field = quote! {
    //       #(#docs)*
    //       #(#serde_attrs)*
    //       pub #ident: #ty
    //     };
    //     let field_init = quote! {
    //       #ident: #init
    //     };
    //     let field_validated_init = quote! {
    //       #ident: #validated_init
    //     };

    //     (field, field_init, field_validated_init)
    //   })
    //   .multiunzip();

    let fields_validation = self.0.fields.iter().filter(|f| f.validate).map(|f| {
      let ident = &f.ident;
      let variant_ident = &f.variant_ident;
      quote! { .validate_with(&self.#ident, #invalidity_ident::#variant_ident) }
    });

    // let required_validation = self.0.fields.iter().filter(|f| f.required).map(|f| {
    //   let ident = &f.ident;
    //   quote! { .invalidate_if(self.#ident.is_none(), #invalidity_ident::Incomplete) }
    // });

    // let builder_doc = format!("Create a new [{ident}].");
    // let into_builder_doc = format!("Turn this entity back into a [{ident}].");

    tokens.extend(quote! {
      // #[derive(Debug, Default, ::serde::Deserialize)]
      // #vis struct #ident #generics {
      //   #(#fields,)*
      // }

      // impl #generics #entity_ident #generics {
      //   #[doc = #builder_doc]
      //   pub fn builder() -> #ident #generics {
      //     <#ident as ::std::default::Default>::default()
      //   }

      //   #[doc = #into_builder_doc]
      //   pub fn into_builder(self) -> #ident #generics {
      //     #ident {
      //       #(#fields_init,)*
      //     }
      //   }
      // }

      impl #generics ::semval::Validate for #ident #generics {
        type Invalidity = #invalidity_ident;

        fn validate(&self) -> ::semval::ValidationResult<Self::Invalidity> {
          ::semval::context::Context::new()
            #(#fields_validation)*
            // #(#required_validation)*
            .into_result()
        }
      }

      // impl #generics #ident #generics {
      //   pub fn build(self) -> ::std::result::Result<#entity_ident #generics, ::semval::context::Context<#invalidity_ident>> {
      //     <Self as ::semval::Validate>::validate(&self)?;
      //     Ok(#entity_ident {
      //       #(#validated_inits,)*
      //     })
      //   }
      // }

      // impl #generics ::std::convert::TryFrom<#ident #generics> for #entity_ident #generics {
      //   type Error = ::error_stack::Report<::hass_mqtt_discovery::validation::ValidationError<::semval::context::Context<#invalidity_ident>>>;

      //   fn try_from(builder: #ident #generics) -> ::std::result::Result<Self, Self::Error> {
      //     builder.build().map_err(|i| ::error_stack::Report::new(::hass_mqtt_discovery::validation::ValidationError::new(i)))
      //   }
      // }
    });
  }
}

pub(super) fn validation(entity: &EntityStruct) -> impl ToTokens + '_ {
  ValidationImpl(entity)
}
