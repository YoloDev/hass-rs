use ahash::HashMap;

#[derive(Clone, PartialEq, Eq)]
struct StringKey {
	start: u16,
	end: u16,
	#[cfg(debug_assertions)]
	_value: String,
}

impl StringKey {
	const EMPTY: StringKey = StringKey {
		start: 0,
		end: 0,
		#[cfg(debug_assertions)]
		_value: String::new(),
	};
}

#[derive(Clone)]
pub struct RouteKey {
	node: u16,
	value: u16,
}

#[derive(Clone, Copy)]
struct ChildrenKey {
	start: u16,
	end: u16,
}

impl ChildrenKey {
	fn new(start: u16) -> Self {
		ChildrenKey { start, end: start }
	}

	fn get<'a, T>(&self, router: &'a Router<T>) -> &'a [Node] {
		&router.nodes[self.start as usize..self.end as usize]
	}
}

#[derive(Clone, Copy)]
struct ValuesKey(u16);

impl ValuesKey {
	fn new() -> Self {
		ValuesKey(0)
	}

	fn get<'a, T>(&self, router: &'a Router<T>) -> &'a [Value<T>] {
		match self.0 {
			0 => &[],
			idx => &router.values[(idx - 1) as usize],
		}
	}

	fn insert<T>(&mut self, values: &mut Vec<Vec<Value<T>>>, value: Value<T>) {
		match self.0 {
			0 => {
				self.0 = values.len() as u16 + 1;
				values.push(vec![value]);
			}
			idx => {
				values[(idx - 1) as usize].push(value);
			}
		}
	}

	fn remove<T>(&mut self, values: &mut [Vec<Value<T>>], key: u16) -> Option<T> {
		match self.0 {
			0 => None,
			idx => {
				let idx = (idx - 1) as usize;
				let values = &mut values[idx];
				let idx = values.iter().position(|v| v.key == key)?;
				Some(values.swap_remove(idx).value)
			}
		}
	}
}

#[derive(Default)]
struct StringInterner {
	strings: String,
	indices: HashMap<String, StringKey>,
}

pub struct Router<T> {
	strings: StringInterner,
	nodes: Vec<Node>,
	values: Vec<Vec<Value<T>>>,
	node_counter: u16,
	value_counter: u16,
}

struct Node {
	key: u16,
	segment: StringKey,
	children: ChildrenKey,
	values: ValuesKey,
}

struct Value<T> {
	key: u16,
	value: T,
}

impl<T> Value<T> {
	fn new(key: u16, value: T) -> Self {
		Value { key, value }
	}
}

impl StringInterner {
	fn get(&self, value: &str) -> Option<StringKey> {
		self.indices.get(value).cloned()
	}

	fn get_or_intern(&mut self, value: &str) -> StringKey {
		if let Some(idx) = self.get(value) {
			return idx;
		}

		let start = self.strings.len();
		self.strings.push_str(value);
		let end = self.strings.len();
		let key = StringKey {
			start: start as u16,
			end: end as u16,
			#[cfg(debug_assertions)]
			_value: value.to_string(),
		};
		self.indices.insert(value.to_string(), key.clone());
		key
	}
}

impl<T> Default for Router<T> {
	fn default() -> Self {
		Self::new()
	}
}

impl<T> Router<T> {
	pub fn new() -> Self {
		let root = Node {
			key: 0,
			segment: StringKey::EMPTY,
			children: ChildrenKey::new(1),
			values: ValuesKey::new(),
		};

		Self {
			strings: StringInterner::default(),
			nodes: vec![root],
			values: Vec::new(),
			node_counter: 1,
			value_counter: 0,
		}
	}

	pub fn insert(&mut self, pattern: &(impl AsRef<str> + ?Sized), value: T) -> RouteKey {
		let pattern = pattern.as_ref();
		let mut node = 0usize;

		for segment in pattern.split('/') {
			let literal = self.strings.get_or_intern(segment);
			node = self.get_or_insert(node, literal);
		}

		let node = &mut self.nodes[node];
		let value_key = self.value_counter;
		let node_key = node.key;
		self.value_counter += 1;
		node
			.values
			.insert(&mut self.values, Value::new(value_key, value));

		RouteKey {
			node: node_key,
			value: value_key,
		}
	}

	pub fn remove(&mut self, key: RouteKey) -> Option<T> {
		let node = self.nodes.iter_mut().find(|n| n.key == key.node)?;
		node.values.remove(&mut self.values, key.value)
	}

	fn node<'a>(&'a self, pattern: &'a str) -> Option<&'a Node> {
		let mut node = &self.nodes[0];
		for segment in pattern.split('/') {
			let Some(literal) = self.strings.get(segment) else {
				return None;
			};

			let children = node.children.get(self);
			let Some(child) = children.iter().find(|child| child.segment == literal) else {
				return None;
			};

			node = child;
		}

		Some(node)
	}

	pub fn matches_with_keys<'a>(
		&'a self,
		pattern: &'a (impl AsRef<str> + ?Sized),
	) -> impl Iterator<Item = (RouteKey, &'a T)> + ExactSizeIterator {
		let (node_key, values) = match self.node(pattern.as_ref()) {
			Some(node) => (node.key, node.values.get(self)),
			None => (0, &[] as &[Value<T>]),
		};

		values.iter().map(move |v| {
			(
				RouteKey {
					node: node_key,
					value: v.key,
				},
				&v.value,
			)
		})
	}

	fn get_or_insert(&mut self, node_idx: usize, segment: StringKey) -> usize {
		let node = &self.nodes[node_idx];
		let children_start = node.children.start as usize;
		let children_end = node.children.end as usize;
		let children = &self.nodes[children_start..children_end];

		for (idx, child) in children.iter().enumerate() {
			if child.segment == segment {
				return children_start + idx;
			}
		}

		// bump end index
		self.nodes[node_idx].children.end += 1;

		// bump all indices after offset
		for node in self.nodes.iter_mut().skip(node_idx + 1) {
			node.children.start += 1;
			node.children.end += 1;
		}

		let node_key = self.node_counter;
		self.node_counter += 1;
		let new_node = Node {
			key: node_key,
			segment,
			children: ChildrenKey::new(self.nodes.len() as u16 + 1),
			values: ValuesKey::new(),
		};

		self.nodes.insert(children_end, new_node);
		children_end
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn basic_test() {
		let mut router = Router::new();
		router.insert("app/default/light/bedroom/brightness", 1);
		router.insert("app/default/light/bedroom/temperature", 2);
		router.insert("app/default/light/bedroom/brightness", 3);
		router.insert("app/default/light/bedroom/temperature", 4);

		// Note: order is not guaranteed after a remove
		assert_eq!(
			router
				.matches_with_keys("app/default/light/bedroom/brightness")
				.map(|(_, v)| v)
				.copied()
				.collect::<Vec<_>>(),
			vec![1, 3]
		);
		assert_eq!(
			router
				.matches_with_keys("app/default/light/bedroom/temperature")
				.map(|(_, v)| v)
				.copied()
				.collect::<Vec<_>>(),
			vec![2, 4]
		);
	}
}
