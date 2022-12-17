use proc_macro2::Span;
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::Token;

#[derive(Copy, Clone)]
pub struct Args {
	pub impl_eq: bool,
}

mod kw {
	syn::custom_keyword!(Eq);
}

impl Parse for Args {
	fn parse(input: ParseStream) -> Result<Self> {
		match try_parse(input) {
			Ok(args) if input.is_empty() => Ok(args),
			_ => Err(error(input.span())),
		}
	}
}

fn try_parse(input: ParseStream) -> Result<Args> {
	if input.peek(Token![?]) {
		input.parse::<Token![?]>()?;
		input.parse::<kw::Eq>()?;
		Ok(Args { impl_eq: false })
	} else {
		Ok(Args { impl_eq: true })
	}
}

fn error(span: Span) -> Error {
	let msg = "only valid argument is ?Eq";
	Error::new(span, msg)
}
