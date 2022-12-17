use super::DocumentStruct;
use crate::util::ModifyLifetimes;
use darling::ToTokens;
use proc_macro2::TokenStream;
use quote::quote;

struct InvalidityEnum<'a>(&'a DocumentStruct);

impl<'a> ToTokens for InvalidityEnum<'a> {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let vis = &self.0.vis;
		let ident = &self.0.invalidity_ident;
		let variants = self.0.fields.iter().filter_map(|f| {
			f.validate.then(|p| {
				let ty = f.ty.make_lifetimes_static();
				let variant_ident = &f.variant_ident;

				match p {
					None => quote! { #variant_ident(<#ty as ::semval::Validate>::Invalidity) },
					Some(p) => {
						quote! { #variant_ident(<#p as crate::validation::Validator<#ty>>::Invalidity) }
					}
				}
			})
		});
		let extra_variants = match self.0.additional_invalidities.as_ref() {
			None => quote! {},
			Some(invalidities) => {
				let variants = invalidities.variants();
				quote! {#(#variants,)*}
			}
		};

		tokens.extend(quote! {
			#[derive(Copy, Clone, Debug, Eq, PartialEq)]
			#vis enum #ident {
				#(#variants,)*
				#extra_variants
			}
		})
	}
}

pub(super) fn invalidity_enum(doc: &DocumentStruct) -> impl ToTokens + '_ {
	InvalidityEnum(doc)
}
