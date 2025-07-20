use generational_arena::{Arena, Index};
use std::{
	collections::{BTreeMap, btree_map},
	ops,
	sync::Arc,
};

#[derive(Debug)]
struct Node<T> {
	route: Arc<str>,
	value: T,
	id: Index,
}

#[derive(Debug)]
struct Nodes<T> {
	route: Arc<str>,
	nodes: Vec<Index>,
	data: T,
}

impl<T> Nodes<T> {
	fn push(&mut self, id: Index) {
		self.nodes.push(id)
	}

	fn remove(&mut self, id: Index) -> Option<Index> {
		let index = self.nodes.iter().position(|node| *node == id)?;
		Some(self.nodes.swap_remove(index))
	}

	fn is_empty(&self) -> bool {
		self.nodes.is_empty()
	}
}

impl<T> Node<T> {
	pub fn new(route: Arc<str>, value: T, id: Index) -> Self {
		Self { route, value, id }
	}
}

#[derive(Debug)]
pub struct Router<R, T> {
	arena: Arena<Node<T>>,
	routes: BTreeMap<Arc<str>, Nodes<R>>,
}

impl<R, T> Default for Router<R, T> {
	fn default() -> Self {
		Self {
			arena: Arena::new(),
			routes: BTreeMap::new(),
		}
	}
}

impl<R, T> Router<R, T> {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn entry(&mut self, route: Arc<str>) -> RouterEntry<'_, R, T> {
		match self.routes.entry(route) {
			btree_map::Entry::Occupied(inner) => RouterEntry::Occupied(OccupiedRouterEntry {
				arena: &mut self.arena,
				inner,
			}),
			btree_map::Entry::Vacant(inner) => RouterEntry::Vacant(VacantRouterEntry {
				arena: &mut self.arena,
				inner,
			}),
		}
	}

	pub fn remove(&mut self, id: Index) -> Option<(T, Option<R>)> {
		let node = self.arena.remove(id)?;
		let nodes = self.routes.get_mut(&node.route)?;
		nodes.remove(id).unwrap();

		if nodes.is_empty() {
			let route = nodes.route.clone();
			let route = self.routes.remove(&route).unwrap();
			Some((node.value, Some(route.data)))
		} else {
			Some((node.value, None))
		}
	}
}

pub struct OccupiedRouterEntry<'a, R, T> {
	arena: &'a mut Arena<Node<T>>,
	inner: btree_map::OccupiedEntry<'a, Arc<str>, Nodes<R>>,
}

impl<'a, R, T> OccupiedRouterEntry<'a, R, T> {
	pub fn insert(mut self, value: T) -> Index {
		let key = self.inner.key().clone();
		let id = self.arena.insert_with(|id| Node::new(key, value, id));

		self.inner.get_mut().push(id);
		id
	}
}

pub struct VacantRouterEntry<'a, R, T> {
	arena: &'a mut Arena<Node<T>>,
	inner: btree_map::VacantEntry<'a, Arc<str>, Nodes<R>>,
}

impl<'a, R, T> VacantRouterEntry<'a, R, T> {
	pub fn insert(self, data: R, value: T) -> Index {
		let key = self.inner.key().clone();
		let nodes = self.inner.insert(Nodes {
			route: key.clone(),
			nodes: Vec::new(),
			data,
		});

		let id = self.arena.insert_with(|id| Node::new(key, value, id));

		nodes.push(id);
		id
	}
}

pub enum RouterEntry<'a, R, T> {
	Occupied(OccupiedRouterEntry<'a, R, T>),
	Vacant(VacantRouterEntry<'a, R, T>),
}

pub struct Match<'a, T>(&'a Node<T>);

impl<'a, T> Match<'a, T> {
	pub fn id(&self) -> Index {
		self.0.id
	}

	// pub fn route(&self) -> &'a str {
	// 	&*self.0.route
	// }

	pub fn value(&self) -> &'a T {
		&self.0.value
	}
}

impl<'a, T> ops::Deref for Match<'a, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.value()
	}
}

impl<'a, T> AsRef<T> for Match<'a, T> {
	fn as_ref(&self) -> &T {
		self
	}
}

impl<R, T> Router<R, T> {
	pub fn matches<'a>(&'a self, key: &str) -> impl ExactSizeIterator<Item = Match<'a, T>> {
		let nodes = match self.routes.get(key) {
			Some(nodes) => nodes.nodes.iter(),
			None => [].iter(),
		};

		nodes.map(|node| Match(&self.arena[*node]))
	}
}

// #[cfg(test)]
// mod tests {
// 	use super::*;

// 	#[test]
// 	fn basic_test() {
// 		let mut router = Router::new();
// 		let r1 = router.insert("app/default/light/bedroom/brightness", 1);
// 		let r2 = router.insert("app/default/light/bedroom/temperature", 2);
// 		let r3 = router.insert("app/default/light/bedroom/brightness", 3);
// 		let r4 = router.insert("app/default/light/bedroom/temperature", 4);

// 		// Note: order is not guaranteed after a remove
// 		assert_eq!(
// 			router
// 				.matches("app/default/light/bedroom/brightness")
// 				.map(|m| *m)
// 				.collect::<Vec<_>>(),
// 			vec![1, 3]
// 		);
// 		assert_eq!(
// 			router
// 				.matches("app/default/light/bedroom/temperature")
// 				.map(|m| *m)
// 				.collect::<Vec<_>>(),
// 			vec![2, 4]
// 		);

// 		assert_eq!(router.remove(r1), Some((1, None)));
// 		assert_eq!(router.remove(r2), Some((2, None)));
// 		assert_eq!(
// 			router.remove(r3),
// 			Some((3, Some("app/default/light/bedroom/brightness".into())))
// 		);
// 		assert_eq!(
// 			router.remove(r4),
// 			Some((4, Some("app/default/light/bedroom/temperature".into())))
// 		);
// 	}
// }
