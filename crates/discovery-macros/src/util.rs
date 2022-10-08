use proc_macro2::Span;
use std::borrow::Cow;
use syn::{
  Constraint, GenericArgument, Lifetime, Path, PathArguments, PathSegment, QSelf, TraitBound, Type,
  TypeArray, TypeGroup, TypeParamBound, TypeParen, TypePath, TypeReference, TypeSlice, TypeTuple,
};

fn static_lifetime() -> Lifetime {
  Lifetime::new("'static", Span::call_site())
}

pub(crate) trait StripLifetimes: Clone {
  fn make_lifetimes_static<'a>(&'a self) -> Cow<'a, Self>;
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

impl StripLifetimes for Type {
  fn make_lifetimes_static<'a>(&'a self) -> Cow<'a, Self> {
    match self {
      Type::Array(v) => v.make_lifetimes_static().or_original(self),
      Type::Group(v) => v.make_lifetimes_static().or_original(self),
      Type::Paren(v) => v.make_lifetimes_static().or_original(self),
      Type::Path(v) => v.make_lifetimes_static().or_original(self),
      Type::Reference(v) => v.make_lifetimes_static().or_original(self),
      Type::Slice(v) => v.make_lifetimes_static().or_original(self),
      Type::Tuple(v) => v.make_lifetimes_static().or_original(self),
      _ => Cow::Borrowed(self),
    }
  }
}

impl StripLifetimes for TypeArray {
  fn make_lifetimes_static<'a>(&'a self) -> Cow<'a, Self> {
    match self.elem.make_lifetimes_static() {
      Cow::Borrowed(_) => Cow::Borrowed(self),
      Cow::Owned(ty) => Cow::Owned(TypeArray {
        elem: Box::new(ty),
        ..self.clone()
      }),
    }
  }
}

impl StripLifetimes for TypeGroup {
  fn make_lifetimes_static<'a>(&'a self) -> Cow<'a, Self> {
    match self.elem.make_lifetimes_static() {
      Cow::Borrowed(_) => Cow::Borrowed(self),
      Cow::Owned(ty) => Cow::Owned(TypeGroup {
        elem: Box::new(ty),
        ..self.clone()
      }),
    }
  }
}

impl StripLifetimes for TypeParen {
  fn make_lifetimes_static<'a>(&'a self) -> Cow<'a, Self> {
    match self.elem.make_lifetimes_static() {
      Cow::Borrowed(_) => Cow::Borrowed(self),
      Cow::Owned(ty) => Cow::Owned(TypeParen {
        elem: Box::new(ty),
        ..self.clone()
      }),
    }
  }
}

impl StripLifetimes for TypeReference {
  fn make_lifetimes_static<'a>(&'a self) -> Cow<'a, Self> {
    match self.lifetime {
      Some(_) => Cow::Owned(TypeReference {
        lifetime: Some(static_lifetime()),
        elem: Box::new(self.elem.make_lifetimes_static().into_owned()),
        ..self.clone()
      }),
      None => match self.elem.make_lifetimes_static() {
        Cow::Borrowed(_) => Cow::Borrowed(self),
        Cow::Owned(ty) => Cow::Owned(TypeReference {
          elem: Box::new(ty),
          ..self.clone()
        }),
      },
    }
  }
}

impl StripLifetimes for TypeSlice {
  fn make_lifetimes_static<'a>(&'a self) -> Cow<'a, Self> {
    match self.elem.make_lifetimes_static() {
      Cow::Borrowed(_) => Cow::Borrowed(self),
      Cow::Owned(ty) => Cow::Owned(TypeSlice {
        elem: Box::new(ty),
        ..self.clone()
      }),
    }
  }
}

impl StripLifetimes for TypePath {
  fn make_lifetimes_static<'a>(&'a self) -> Cow<'a, Self> {
    let qself_ty = self
      .qself
      .as_ref()
      .map(|v| (v.ty.make_lifetimes_static(), v));

    let path = self.path.make_lifetimes_static();
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

impl StripLifetimes for TypeTuple {
  fn make_lifetimes_static<'a>(&'a self) -> Cow<'a, Self> {
    let elems: Vec<_> = self
      .elems
      .iter()
      .map(|v| v.make_lifetimes_static())
      .collect();
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

impl StripLifetimes for Path {
  fn make_lifetimes_static<'a>(&'a self) -> Cow<'a, Self> {
    fn clone_and_continue(value: &Path, idx: usize, segment: PathSegment) -> Cow<'static, Path> {
      let mut ret = value.clone();
      ret.segments[idx] = segment;
      for segment in ret.segments.iter_mut().skip(idx) {
        match segment.make_lifetimes_static() {
          Cow::Borrowed(_) => {}
          Cow::Owned(stripped) => {
            *segment = stripped;
          }
        }
      }

      Cow::Owned(ret)
    }

    let iter = self.segments.iter().enumerate();
    for (idx, segment) in iter {
      if let Cow::Owned(segment) = segment.make_lifetimes_static() {
        return clone_and_continue(self, idx, segment);
      }
    }

    Cow::Borrowed(self)
  }
}

impl StripLifetimes for PathSegment {
  fn make_lifetimes_static<'a>(&'a self) -> Cow<'a, Self> {
    match &self.arguments {
      PathArguments::AngleBracketed(arguments) => {
        let mut args = Vec::with_capacity(arguments.args.len());
        args.extend(arguments.args.iter().map(|a| match a {
          GenericArgument::Lifetime(_) => Cow::Owned(GenericArgument::Lifetime(static_lifetime())),
          GenericArgument::Type(v) => match v.make_lifetimes_static() {
            Cow::Borrowed(_) => Cow::Borrowed(a),
            Cow::Owned(ty) => Cow::Owned(GenericArgument::Type(ty)),
          },
          GenericArgument::Binding(v) => match v.ty.make_lifetimes_static() {
            Cow::Borrowed(_) => Cow::Borrowed(a),
            Cow::Owned(ty) => {
              Cow::Owned(GenericArgument::Binding(syn::Binding { ty, ..v.clone() }))
            }
          },
          GenericArgument::Constraint(v) => match v.make_lifetimes_static() {
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

impl StripLifetimes for Constraint {
  fn make_lifetimes_static<'a>(&'a self) -> Cow<'a, Self> {
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
