use generational_arena::{Arena, Index};
use std::{collections::BTreeMap, ops, rc::Rc};

#[derive(Debug)]
struct Node<T> {
	route: Rc<str>,
	value: T,
	id: Index,
}

#[derive(Debug)]
struct Nodes {
	route: Rc<str>,
	nodes: Vec<Index>,
}

impl Nodes {
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

// impl<'a, T> IntoIterator for &'a Nodes<T> {
// 	type Item = &'a Node<T>;
// 	type IntoIter = std::slice::Iter<'a, Node<T>>;

// 	fn into_iter(self) -> Self::IntoIter {
// 		self.nodes.iter()
// 	}
// }

impl<T> Node<T> {
	pub fn new(route: Rc<str>, value: T, id: Index) -> Self {
		Self { route, value, id }
	}
}

#[derive(Debug)]
pub struct Router<T> {
	arena: Arena<Node<T>>,
	routes: BTreeMap<Rc<str>, Nodes>,
}

impl<T> Default for Router<T> {
	fn default() -> Self {
		Self {
			arena: Arena::new(),
			routes: BTreeMap::new(),
		}
	}
}

impl<T> Router<T> {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn insert(&mut self, route: &str, value: T) -> Index {
		let route = Rc::from(route);
		let id = self
			.arena
			.insert_with(|id| Node::new(Rc::clone(&route), value, id));

		let nodes = self
			.routes
			.entry(route.clone())
			.or_insert_with_key(|route| Nodes {
				route: route.clone(),
				nodes: Vec::new(),
			});

		nodes.push(id);
		id
	}

	pub fn remove(&mut self, id: Index) -> Option<(T, Option<Rc<str>>)> {
		let node = self.arena.remove(id)?;
		let nodes = self.routes.get_mut(&node.route)?;
		nodes.remove(id).unwrap();

		if nodes.is_empty() {
			let route = nodes.route.clone();
			self.routes.remove(&route);
			Some((node.value, Some(route)))
		} else {
			Some((node.value, None))
		}
	}
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

impl<T> Router<T> {
	pub fn matches<'a>(
		&'a self,
		key: &str,
	) -> impl Iterator<Item = Match<'a, T>> + ExactSizeIterator {
		let nodes = match self.routes.get(key) {
			Some(nodes) => nodes.nodes.iter(),
			None => [].iter(),
		};

		nodes.map(|node| Match(&self.arena[*node]))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn basic_test() {
		let mut router = Router::new();
		let r1 = router.insert("app/default/light/bedroom/brightness", 1);
		let r2 = router.insert("app/default/light/bedroom/temperature", 2);
		let r3 = router.insert("app/default/light/bedroom/brightness", 3);
		let r4 = router.insert("app/default/light/bedroom/temperature", 4);

		// Note: order is not guaranteed after a remove
		assert_eq!(
			router
				.matches("app/default/light/bedroom/brightness")
				.map(|m| *m)
				.collect::<Vec<_>>(),
			vec![1, 3]
		);
		assert_eq!(
			router
				.matches("app/default/light/bedroom/temperature")
				.map(|m| *m)
				.collect::<Vec<_>>(),
			vec![2, 4]
		);

		assert_eq!(router.remove(r1), Some((1, None)));
		assert_eq!(router.remove(r2), Some((2, None)));
		assert_eq!(
			router.remove(r3),
			Some((3, Some("app/default/light/bedroom/brightness".into())))
		);
		assert_eq!(
			router.remove(r4),
			Some((4, Some("app/default/light/bedroom/temperature".into())))
		);
	}
}
