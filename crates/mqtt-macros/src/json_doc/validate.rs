use super::DocumentStruct;
use darling::ToTokens;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};

struct ValidationImpl<'a>(&'a DocumentStruct);

impl<'a> ToTokens for ValidationImpl<'a> {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let generics = &self.0.generics;
		let ident = &self.0.ident;
		let invalidity_ident = &self.0.invalidity_ident;

		let fields_validation = self.0.fields.iter().filter_map(|f| {
			f.validate.then(|span, p| {
				let ident = format_ident!("{}", &f.ident, span = Span::call_site());
				let variant_ident = &f.variant_ident;
				let field = quote!(&self.#ident);
				match p {
					None => quote! { .validate_with(#field, #invalidity_ident::#variant_ident) },
					Some(p) => {
						let p = quote_spanned!(*span=>#p);
						quote! { .validate_using_with(&#p, #field, #invalidity_ident::#variant_ident) }
					}
				}
			})
		});

		let extra_validation = match self.0.additional_invalidities.as_ref() {
			None => quote! {},
			Some(_) => quote! { .validate_using(self, self) },
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

pub(super) fn validation(doc: &DocumentStruct) -> impl ToTokens + '_ {
	ValidationImpl(doc)
}
