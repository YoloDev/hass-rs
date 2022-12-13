#![cfg_attr(provide_any, feature(provide_any))]
#![cfg_attr(provide_any, feature(error_generic_member_access))]

use std::{error::Error, fmt};

#[cfg(provide_any)]
use std::any::Demand;

pub struct DynError(Box<dyn Error + Send + Sync + 'static>);

impl DynError {
	pub fn new<E: Error + Send + Sync + 'static>(error: E) -> Self {
		Self(Box::new(error))
	}
}

impl fmt::Debug for DynError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(&*self.0, f)
	}
}

impl fmt::Display for DynError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Display::fmt(&*self.0, f)
	}
}

impl Error for DynError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		self.0.source()
	}

	#[cfg(provide_any)]
	fn provide<'a>(&'a self, demand: &mut Demand<'a>) {
		self.0.provide(demand);
	}
}
