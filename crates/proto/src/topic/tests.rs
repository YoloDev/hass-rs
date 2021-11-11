use crate::Topic;

#[test]
fn parent() {
  let path = Topic::new("foo/bar/baz");

  let parent = path.parent().unwrap();
  assert_eq!(parent, Topic::new("foo/bar"));

  let grand_parent = parent.parent().unwrap();
  assert_eq!(grand_parent, Topic::new("foo"));
  assert_eq!(grand_parent.parent(), None);
}
