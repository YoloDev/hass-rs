use super::EntityStruct;
use darling::ToTokens;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};

struct ValidationImpl<'a>(&'a EntityStruct);

impl<'a> ToTokens for ValidationImpl<'a> {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let generics = &self.0.generics;
		let ident = &self.0.ident;
		let invalidity_ident = &self.0.invalidity_ident;

		let fields_validation = self.0.fields.iter().filter_map(|f| {
			f.validate.then(|p| {
				let ident = format_ident!("{}", &f.ident, span = Span::call_site());
				let variant_ident = &f.variant_ident;
				let field = quote!(&self.#ident);
				let field = p.map(|p| quote!(&#p(#field))).unwrap_or(field);
				quote! { .validate_with(#field, #invalidity_ident::#variant_ident) }
			})
		});

		let extra_validation = match self.0.additional_invalidities.as_ref() {
			None => quote! {},
			Some(_) => quote! { .validate_entity(self) },
		};

		tokens.extend(quote! {
			impl #generics ::semval::Validate for #ident #generics {
				type Invalidity = #invalidity_ident;

				fn validate(&self) -> ::semval::ValidationResult<Self::Invalidity> {
					#[allow(unused)]
					use crate::validation::ValidateContextExt;

					::semval::context::Context::new()
						#(#fields_validation)*
						#extra_validation
						.into_result()
				}
			}
		});
	}
}

pub(super) fn validation(entity: &EntityStruct) -> impl ToTokens + '_ {
	ValidationImpl(entity)
}
