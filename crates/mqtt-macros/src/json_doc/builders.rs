use super::DocumentStruct;
use darling::ToTokens;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{Path, PathArguments, Type};

struct Builders<'a>(&'a DocumentStruct);

fn match_path<'a>(path: &'a Path, segments: &[&str]) -> Option<&'a PathArguments> {
	if path.segments.len() == 1 && path.segments[0].ident == segments.last().unwrap() {
		Some(&path.segments[0].arguments)
	} else if path.leading_colon.is_some()
		&& path.segments.len() == segments.len()
		&& path
			.segments
			.iter()
			.zip(segments)
			.all(|(a, b)| a.ident == b)
	{
		Some(&path.segments.last().unwrap().arguments)
	} else {
		None
	}
}

fn as_option(p: &Path) -> Option<&Type> {
	match_path(p, &["std", "option", "Option"])
		.or_else(|| match_path(p, &["core", "option", "Option"]))
		.and_then(|args| {
			if let PathArguments::AngleBracketed(args) = args
				&& args.args.len() == 1
				&& let syn::GenericArgument::Type(t) = &args.args[0]
			{
				Some(t)
			} else {
				None
			}
		})
}

impl<'a> ToTokens for Builders<'a> {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let generics = &self.0.generics;
		let ident = &self.0.ident;

		let builders = self.0.fields.iter().filter(|f| f.builder.enabled).map(|f| {
			let ident = f
				.builder
				.rename
				.clone()
				.unwrap_or_else(|| format_ident!("{}", &f.ident, span = f.builder.span));
			let docs = &f.docs;
			let ty = &f.ty;
			match ty {
				syn::Type::Path(p) => {
					if let Some(inner) = as_option(&p.path) {
						let unset_ident = format_ident!("unset_{}", ident, span = Span::call_site());
						quote! {
							#(#docs)*
							pub fn #ident(mut self, #ident: impl ::core::convert::Into< #inner >) -> Self {
								self.#ident = Some(#ident.into());
								self
							}

							#(#docs)*
							pub fn #unset_ident(&mut self) -> &mut Self {
								self.#ident = None;
								self
							}
						}
					} else {
						quote! {
							#(#docs)*
							pub fn #ident(mut self, #ident: impl ::core::convert::Into< #ty >) -> Self {
								self.#ident = #ident.into();
								self
							}
						}
					}
				}
				// TODO: deal with?
				_ => panic!("type should be a path"),
			}
		});

		tokens.extend(quote! {
			impl #generics #ident #generics {
				#(#builders)*
			}
		});
	}
}

pub(super) fn builders(doc: &DocumentStruct) -> impl ToTokens + '_ {
	Builders(doc)
}
