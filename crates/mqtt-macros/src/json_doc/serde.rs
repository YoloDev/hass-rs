use super::DocumentStruct;
use crate::util::ModifyLifetimes;
use darling::ToTokens;
use itertools::MultiUnzip;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};

struct SerdeImpl<'a>(&'a DocumentStruct);

impl<'a> ToTokens for SerdeImpl<'a> {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let generics = &self.0.generics;
		let ident = &self.0.ident;
		let proxy_ident = format_ident!("{}Proxy", &self.0.ident, span = Span::call_site());

		let mut proxy_generics = generics.clone();
		proxy_generics.params.clear();
		proxy_generics.params.push(syn::parse_quote!('b));
		proxy_generics.params.push(syn::parse_quote!('p));

		let proxy_inner_lifetime = proxy_generics.lifetimes().next().unwrap();
		let proxy_outer_lifetime = proxy_generics.lifetimes().last().unwrap();

		let proxy_fields = self.0.fields.iter().map(|f| {
			let ident = format_ident!("{}", &f.ident, span = Span::call_site());
			let attrs = &f.attrs;
			let ty = f.ty.make_lifetimes(&proxy_inner_lifetime.lifetime);
			quote! {
				#(#attrs)*
				#ident: & #proxy_outer_lifetime #ty
			}
		});

		let (ser_fns, additional_proxy_fields, additional_proxy_assigns) =
			match self.0.additional_props.as_ref() {
				None => (quote! {}, quote! {}, quote! {}),
				Some(v) => {
					let (fns, flds, assigns): (Vec<_>, Vec<_>, Vec<_>) = v
						.props()
						.enumerate()
						.map(|(idx, (name, value))| {
							let ident = format_ident!("__const_field_{}", idx);
							let ser_ident = format_ident!("__serialize_field_{}", idx);
							let ser_ident_str = ser_ident.to_string();
							let ser_fn = quote! {
								fn #ser_ident<S: ::serde::Serializer>(_: &(), s: S) -> ::std::result::Result<S::Ok, S::Error> {
									s.serialize_str(#value)
								}
							};
							let fld = quote! {
								#[serde(skip_deserializing, serialize_with = #ser_ident_str, rename = #name)]
								#ident: ()
							};
							let assign = quote! { #ident: () };

							(ser_fn, fld, assign)
						})
						.multiunzip();

					(
						quote! { #(#fns)* },
						quote! { #(#flds,)* },
						quote! { #(#assigns,)* },
					)
				}
			};

		let proxy_assign = self.0.fields.iter().map(|f| {
			let ident = format_ident!("{}", &f.ident, span = Span::call_site());
			quote! {
				#ident: &doc.#ident
			}
		});

		tokens.extend(quote! {
      impl #generics crate::Document for #ident #generics {
        fn serialize_validated<S>(validated: ::semval::Validated::<& #ident #generics>, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
        where
          S: ::serde::Serializer,
        {
          #ser_fns

          #[derive(::serde::Serialize)]
          struct #proxy_ident #proxy_generics {
            #(#proxy_fields,)*
            #additional_proxy_fields
          }

          let doc = *validated;
          let proxy = #proxy_ident {
            #(#proxy_assign,)*
            #additional_proxy_assigns
          };

          <#proxy_ident as ::serde::Serialize>::serialize(
            &proxy,
            serializer,
          )
        }
      }

      impl #generics ::serde::Serialize for #ident #generics {
        #[inline]
        fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
        where
          S: ::serde::Serializer,
        {
          crate::Document::serialize(self, serializer)
        }
      }
    });
	}
}

pub(super) fn serde_impl(doc: &DocumentStruct) -> impl ToTokens + '_ {
	SerdeImpl(doc)
}
