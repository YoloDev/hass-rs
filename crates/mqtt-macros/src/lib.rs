mod args;
mod entity_doc;
mod json_doc;
mod state_doc;
mod util;

use args::Args;
use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro_attribute]
pub fn entity_document(attr: TokenStream, item: TokenStream) -> TokenStream {
	let args = parse_macro_input!(attr as Args);
	match entity_doc::create(item.into(), args) {
		Ok(tokens) => tokens.into(),
		Err(err) => err.write_errors().into(),
	}
}

#[proc_macro_attribute]
pub fn state_document(attr: TokenStream, item: TokenStream) -> TokenStream {
	let args = parse_macro_input!(attr as Args);
	match state_doc::create(item.into(), args) {
		Ok(tokens) => tokens.into(),
		Err(err) => err.write_errors().into(),
	}
}
