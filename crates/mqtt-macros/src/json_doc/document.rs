use super::DocumentStruct;
use darling::ToTokens;
use proc_macro2::TokenStream;
use quote::quote;

struct Document<'a>(&'a DocumentStruct);

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
			let ty = &f.ty;
			quote! {
				#(#docs)*
				#(#attrs)*
				pub #ident: #ty
			}
		});

		tokens.extend(quote! {
			#(#docs)*
			#(#attrs)*
			#[derive(Debug, Clone, PartialEq, Eq, ::serde::Deserialize)]
			#vis struct #ident #generics {
				#(#fields,)*
			}
		});
	}
}

pub(super) fn document_struct(doc: &DocumentStruct) -> impl ToTokens + '_ {
	Document(doc)
}
