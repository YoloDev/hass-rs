use proc_macro2::Span;
use quote::quote;
use std::{borrow::Cow, mem};
use syn::{
	Attribute, Constraint, GenericArgument, Lifetime, Path, PathArguments, PathSegment, QSelf, Type,
	TypeArray, TypeGroup, TypeParamBound, TypeParen, TypePath, TypeReference, TypeSlice, TypeTuple,
};

pub trait CfgExt {
	fn cfg(&self, cfg: impl quote::ToTokens) -> proc_macro2::TokenStream;
}

impl CfgExt for Attribute {
	fn cfg(&self, cfg: impl quote::ToTokens) -> proc_macro2::TokenStream {
		let meta = &self.meta;
		quote!(#[cfg_attr(#cfg, #meta)])
	}
}

pub(crate) trait Prepend {
	fn prepend(&mut self, items: Self);
}

impl Prepend for syn::FieldsNamed {
	fn prepend(&mut self, items: Self) {
		let mut items = items.named;
		mem::swap(&mut self.named, &mut items);

		if !items.is_empty() {
			if let Some(l) = self.named.last_mut() {
				if l.colon_token.is_none() {
					l.colon_token = Some(<syn::Token!(:)>::default());
				}
			}
		}

		self.named.extend(items);
	}
}
pub(crate) trait ModifyLifetimes: Clone {
	fn map_lifetimes<'a>(&'a self, f: &mut impl FnMut(&Lifetime) -> Lifetime) -> Cow<'a, Self>;

	fn make_lifetimes_static(&self) -> Cow<Self> {
		self.make_lifetimes(&Lifetime::new("'static", Span::call_site()))
	}
	fn make_lifetimes(&self, lifetime: &Lifetime) -> Cow<Self> {
		self.map_lifetimes(&mut |_| lifetime.clone())
	}
}

trait TypeVariantCowExt<'a>: Clone {
	fn or_original(self, original: &'a Type) -> Cow<'a, Type>;
}

impl<'a, T: Into<Type> + Clone> TypeVariantCowExt<'a> for Cow<'a, T> {
	fn or_original(self, original: &'a Type) -> Cow<'a, Type> {
		match self {
			Cow::Borrowed(_) => Cow::Borrowed(original),
			Cow::Owned(ty) => Cow::Owned(ty.into()),
		}
	}
}

impl ModifyLifetimes for Type {
	fn map_lifetimes<'a>(&'a self, f: &mut impl FnMut(&Lifetime) -> Lifetime) -> Cow<'a, Self> {
		match self {
			Type::Array(v) => v.map_lifetimes(f).or_original(self),
			Type::Group(v) => v.map_lifetimes(f).or_original(self),
			Type::Paren(v) => v.map_lifetimes(f).or_original(self),
			Type::Path(v) => v.map_lifetimes(f).or_original(self),
			Type::Reference(v) => v.map_lifetimes(f).or_original(self),
			Type::Slice(v) => v.map_lifetimes(f).or_original(self),
			Type::Tuple(v) => v.map_lifetimes(f).or_original(self),
			_ => Cow::Borrowed(self),
		}
	}
}

impl ModifyLifetimes for TypeArray {
	fn map_lifetimes<'a>(&'a self, f: &mut impl FnMut(&Lifetime) -> Lifetime) -> Cow<'a, Self> {
		match self.elem.map_lifetimes(f) {
			Cow::Borrowed(_) => Cow::Borrowed(self),
			Cow::Owned(ty) => Cow::Owned(TypeArray {
				elem: Box::new(ty),
				..self.clone()
			}),
		}
	}
}

impl ModifyLifetimes for TypeGroup {
	fn map_lifetimes<'a>(&'a self, f: &mut impl FnMut(&Lifetime) -> Lifetime) -> Cow<'a, Self> {
		match self.elem.map_lifetimes(f) {
			Cow::Borrowed(_) => Cow::Borrowed(self),
			Cow::Owned(ty) => Cow::Owned(TypeGroup {
				elem: Box::new(ty),
				..self.clone()
			}),
		}
	}
}

impl ModifyLifetimes for TypeParen {
	fn map_lifetimes<'a>(&'a self, f: &mut impl FnMut(&Lifetime) -> Lifetime) -> Cow<'a, Self> {
		match self.elem.map_lifetimes(f) {
			Cow::Borrowed(_) => Cow::Borrowed(self),
			Cow::Owned(ty) => Cow::Owned(TypeParen {
				elem: Box::new(ty),
				..self.clone()
			}),
		}
	}
}

impl ModifyLifetimes for TypeReference {
	fn map_lifetimes<'a>(&'a self, f: &mut impl FnMut(&Lifetime) -> Lifetime) -> Cow<'a, Self> {
		match &self.lifetime {
			Some(l) => Cow::Owned(TypeReference {
				lifetime: Some(f(l)),
				elem: Box::new(self.elem.map_lifetimes(f).into_owned()),
				..self.clone()
			}),
			None => match self.elem.map_lifetimes(f) {
				Cow::Borrowed(_) => Cow::Borrowed(self),
				Cow::Owned(ty) => Cow::Owned(TypeReference {
					elem: Box::new(ty),
					..self.clone()
				}),
			},
		}
	}
}

impl ModifyLifetimes for TypeSlice {
	fn map_lifetimes<'a>(&'a self, f: &mut impl FnMut(&Lifetime) -> Lifetime) -> Cow<'a, Self> {
		match self.elem.map_lifetimes(f) {
			Cow::Borrowed(_) => Cow::Borrowed(self),
			Cow::Owned(ty) => Cow::Owned(TypeSlice {
				elem: Box::new(ty),
				..self.clone()
			}),
		}
	}
}

impl ModifyLifetimes for TypePath {
	fn map_lifetimes<'a>(&'a self, f: &mut impl FnMut(&Lifetime) -> Lifetime) -> Cow<'a, Self> {
		let qself_ty = self.qself.as_ref().map(|v| (v.ty.map_lifetimes(f), v));

		let path = self.path.map_lifetimes(f);
		match (qself_ty, path) {
			(Some((Cow::Borrowed(_), _)) | None, Cow::Borrowed(_)) => Cow::Borrowed(self),
			(qself_ty, path) => Cow::Owned(TypePath {
				qself: qself_ty.map(|(ty, v)| QSelf {
					ty: Box::new(ty.into_owned()),
					..v.clone()
				}),
				path: path.into_owned(),
			}),
		}
	}
}

impl ModifyLifetimes for TypeTuple {
	fn map_lifetimes<'a>(&'a self, f: &mut impl FnMut(&Lifetime) -> Lifetime) -> Cow<'a, Self> {
		let elems: Vec<_> = self.elems.iter().map(|v| v.map_lifetimes(f)).collect();
		if !elems.iter().any(|v| matches!(v, Cow::Owned(_))) {
			Cow::Borrowed(self)
		} else {
			Cow::Owned(TypeTuple {
				elems: elems.into_iter().map(|v| v.into_owned()).collect(),
				..self.clone()
			})
		}
	}
}

impl ModifyLifetimes for Path {
	fn map_lifetimes<'a>(&'a self, f: &mut impl FnMut(&Lifetime) -> Lifetime) -> Cow<'a, Self> {
		let iter = self.segments.iter().enumerate();
		for (idx, segment) in iter {
			if let Cow::Owned(segment) = segment.map_lifetimes(f) {
				let mut ret = self.clone();
				ret.segments[idx] = segment;
				{
					for segment in ret.segments.iter_mut().skip(idx) {
						match segment.map_lifetimes(f) {
							Cow::Borrowed(_) => {}
							Cow::Owned(stripped) => {
								*segment = stripped;
							}
						}
					}
				}

				return Cow::Owned(ret);
			}
		}

		Cow::Borrowed(self)
	}
}

impl ModifyLifetimes for PathSegment {
	fn map_lifetimes<'a>(&'a self, f: &mut impl FnMut(&Lifetime) -> Lifetime) -> Cow<'a, Self> {
		match &self.arguments {
			PathArguments::AngleBracketed(arguments) => {
				let mut args = Vec::with_capacity(arguments.args.len());
				args.extend(arguments.args.iter().map(|a| match a {
					GenericArgument::Lifetime(l) => Cow::Owned(GenericArgument::Lifetime(f(l))),
					GenericArgument::Type(v) => match v.map_lifetimes(f) {
						Cow::Borrowed(_) => Cow::Borrowed(a),
						Cow::Owned(ty) => Cow::Owned(GenericArgument::Type(ty)),
					},
					GenericArgument::AssocType(v) => match v.ty.map_lifetimes(f) {
						Cow::Borrowed(_) => Cow::Borrowed(a),
						Cow::Owned(ty) => Cow::Owned(GenericArgument::AssocType(syn::AssocType {
							ty,
							..v.clone()
						})),
					},
					GenericArgument::Constraint(v) => match v.map_lifetimes(f) {
						Cow::Borrowed(_) => Cow::Borrowed(a),
						Cow::Owned(ty) => Cow::Owned(GenericArgument::Constraint(ty)),
					},
					_ => Cow::Borrowed(a),
				}));

				if args.len() != arguments.args.len() || args.iter().any(|a| matches!(a, Cow::Owned(_))) {
					let mut ret = self.clone();
					let args = args.into_iter().map(|cow| cow.into_owned()).collect();
					ret.arguments = PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
						args,
						..arguments.clone()
					});

					Cow::Owned(ret)
				} else {
					Cow::Borrowed(self)
				}
			}
			_ => Cow::Borrowed(self),
		}
	}
}

impl ModifyLifetimes for Constraint {
	fn map_lifetimes<'a>(&'a self, _f: &mut impl FnMut(&Lifetime) -> Lifetime) -> Cow<'a, Self> {
		let mut constraints = Vec::with_capacity(self.bounds.len());
		constraints.extend(self.bounds.iter().filter_map(|b| match b {
			TypeParamBound::Lifetime(_) => None,
			_ => Some(Cow::Borrowed(b)),
		}));

		if constraints.len() != self.bounds.len()
			|| constraints.iter().any(|a| matches!(a, Cow::Owned(_)))
		{
			let mut ret = self.clone();
			ret.bounds = constraints
				.into_iter()
				.map(|cow| cow.into_owned())
				.collect();
			Cow::Owned(ret)
		} else {
			Cow::Borrowed(self)
		}
	}
}
