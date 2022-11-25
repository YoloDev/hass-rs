mod entity;
mod util;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn entity_document(_attr: TokenStream, item: TokenStream) -> TokenStream {
  match entity::create(item.into()) {
    Ok(tokens) => tokens.into(),
    Err(err) => err.write_errors().into(),
  }
}
