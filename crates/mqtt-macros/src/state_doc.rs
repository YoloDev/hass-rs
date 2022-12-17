use crate::{args::Args, json_doc::DocumentStruct};
use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::parse2;

struct StateStruct(DocumentStruct);

impl FromDeriveInput for StateStruct {
	fn from_derive_input(input: &syn::DeriveInput) -> darling::Result<Self> {
		let input = input.clone();

		DocumentStruct::from_derive_input(&input).map(Self)
	}
}

impl StateStruct {
	fn into_token_stream(self, args: Args) -> TokenStream {
		let mut tokens = TokenStream::new();
		self.0.document_struct(&args).to_tokens(&mut tokens);
		self.0.ctor().to_tokens(&mut tokens);
		self.0.builders().to_tokens(&mut tokens);
		self.0.invalidity_enum().to_tokens(&mut tokens);
		self.0.validate().to_tokens(&mut tokens);
		self.0.serde().to_tokens(&mut tokens);
		tokens
	}
}

pub fn create(input: TokenStream, args: Args) -> darling::Result<TokenStream> {
	let parsed: syn::DeriveInput = parse2(input)?;
	let doc = StateStruct::from_derive_input(&parsed)?;
	Ok(doc.into_token_stream(args))
}
