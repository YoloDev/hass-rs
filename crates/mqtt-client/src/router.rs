use std::{collections::BTreeMap, fmt, ops, rc::Rc};

pub struct Id<T>(u16, std::marker::PhantomData<T>);

impl<T> fmt::Debug for Id<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "Id({})", self.0)
	}
}

impl<T> Clone for Id<T> {
	fn clone(&self) -> Self {
		Self(self.0, std::marker::PhantomData)
	}
}

impl<T> Copy for Id<T> {}

impl<T> PartialEq for Id<T> {
	fn eq(&self, other: &Self) -> bool {
		self.0 == other.0
	}
}

impl<T> Eq for Id<T> {}

impl<T> PartialOrd for Id<T> {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl<T> Ord for Id<T> {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.0.cmp(&other.0)
	}
}

impl<T> std::hash::Hash for Id<T> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.0.hash(state);
	}
}

#[derive(Debug)]
struct Node<T> {
	route: Rc<str>,
	value: T,
	id: Id<T>,
}

#[derive(Debug)]
struct Nodes<T> {
	route: Rc<str>,
	nodes: Vec<Node<T>>,
}

impl<T> Nodes<T> {
	fn push(&mut self, id: Id<T>, value: T) {
		self.nodes.push(Node::new(self.route.clone(), value, id));
	}

	fn remove(&mut self, id: Id<T>) -> Option<T> {
		let index = self.nodes.iter().position(|node| node.id == id)?;
		Some(self.nodes.swap_remove(index).value)
	}

	fn is_empty(&self) -> bool {
		self.nodes.is_empty()
	}
}

impl<'a, T> IntoIterator for &'a Nodes<T> {
	type Item = &'a Node<T>;
	type IntoIter = std::slice::Iter<'a, Node<T>>;

	fn into_iter(self) -> Self::IntoIter {
		self.nodes.iter()
	}
}

impl<T> Node<T> {
	pub fn new(route: Rc<str>, value: T, id: Id<T>) -> Self {
		Self { route, value, id }
	}
}

#[derive(Debug)]
pub struct Router<T> {
	arena: Vec<Node<T>>,
	routes: BTreeMap<Rc<str>, Nodes<T>>,
}

impl<T> Default for Router<T> {
	fn default() -> Self {
		Self {
			arena: Vec::new(),
			routes: BTreeMap::new(),
		}
	}
}

impl<T> Router<T> {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn insert(&mut self, route: &str, value: T) -> Id<T> {
		let id = Id(self.arena.len() as u16, std::marker::PhantomData);
		let nodes = self
			.routes
			.entry(route.into())
			.or_insert_with_key(|route| Nodes {
				route: route.clone(),
				nodes: Vec::new(),
			});

		nodes.push(id, value);
		id
	}

	pub fn remove(&mut self, id: Id<T>) -> Option<(T, bool)> {
		let node = self.arena.get_mut(id.0 as usize)?;
		let nodes = self.routes.get_mut(&node.route)?;
		let value = nodes.remove(id)?;
		if nodes.is_empty() {
			let route = nodes.route.clone();
			self.routes.remove(&route);
			Some((value, true))
		} else {
			Some((value, false))
		}
	}

	// pub fn matches_with_keys<'a>(
	// 	&'a self,
	// 	key: &str,
	// ) -> impl Iterator<Item = (&'a str, &'a T)> + ExactSizeIterator {
	// 	let nodes = match self.routes.get(key) {
	// 		Some(nodes) => nodes.nodes.iter(),
	// 		None => [].iter(),
	// 	};

	// 	nodes.map(|node| (&*node.route, &node.value))
	// }

	// pub fn matches<'a>(&'a self, key: &str) -> impl Iterator<Item = &'a T> + ExactSizeIterator {
	// 	self.matches_with_keys(key).map(|(_, v)| v)
	// }
}

pub struct Match<'a, T>(&'a Node<T>);

impl<'a, T> Match<'a, T> {
	pub fn id(&self) -> Id<T> {
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

		nodes.map(|node| Match(node))
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

		assert_eq!(router.remove(r1), Some((1, false)));
		assert_eq!(router.remove(r2), Some((2, false)));
		assert_eq!(router.remove(r3), Some((3, true)));
		assert_eq!(router.remove(r4), Some((4, true)));
	}
}
