use super::DocumentStruct;
use darling::ToTokens;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};

struct Ctor<'a>(&'a DocumentStruct);

impl<'a> ToTokens for Ctor<'a> {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let generics = &self.0.generics;
		let ident = &self.0.ident;

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
						quote! { #ident: ::core::convert::Into::into( #ident ) },
						Some(quote! { #ident: impl ::core::convert::Into::< #ty > }),
					)
				} else {
					(quote! { #ident: ::core::default::Default::default() }, None)
				};
				(arg, field)
			})
			.unzip();
		let ctor_args = ctor_args.into_iter().flatten();

		tokens.extend(quote! {
			impl #generics #ident #generics {
				pub fn new(#(#ctor_args,)* ) -> Self {
					Self {
						#(#ctor_fields,)*
					}
				}
			}
		});
	}
}

pub(super) fn ctor(doc: &DocumentStruct) -> impl ToTokens + '_ {
	Ctor(doc)
}
