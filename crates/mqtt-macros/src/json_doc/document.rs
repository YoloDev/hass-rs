use super::DocumentStruct;
use crate::{args::Args, util::CfgExt};
use darling::ToTokens;
use proc_macro2::TokenStream;
use quote::quote;

struct Document<'a>(&'a DocumentStruct, &'a Args);

impl<'a> ToTokens for Document<'a> {
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
			let serde = f.serde.iter().map(|s| s.cfg(quote!(feature = "de")));
			let ty = &f.ty;
			quote! {
				#(#docs)*
				#(#serde)*
				#(#attrs)*
				pub #ident: #ty
			}
		});

		let mut derives = vec![quote!(Debug), quote!(Clone), quote!(PartialEq)];
		if self.1.impl_eq {
			derives.push(quote!(Eq));
		}

		tokens.extend(quote! {
			#(#docs)*
			#(#attrs)*
			#[derive(#(#derives,)*)]
			#[cfg_attr(feature = "de", derive(serde::Deserialize))]
			#vis struct #ident #generics {
				#(#fields,)*
			}
		});
	}
}

pub(super) fn document_struct<'a>(doc: &'a DocumentStruct, args: &'a Args) -> impl ToTokens + 'a {
	Document(doc, args)
}
