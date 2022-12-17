pub trait Dirty {
	fn dirty(&self) -> bool;
}

pub(crate) trait DirtyClear {
	fn clear_dirty(&self) -> bool;
}
