#[cfg(test)]
mod tests;

use std::{
  borrow::{Borrow, Cow},
  cmp,
  error::Error,
  fmt,
  hash::{Hash, Hasher},
  iter::{self, FusedIterator},
  ops,
  rc::Rc,
  str::FromStr,
  sync::Arc,
};

////////////////////////////////////////////////////////////////////////////////
// Exposed parsing helpers
////////////////////////////////////////////////////////////////////////////////

impl Topic {
  /// Topic level separator.
  pub const SEPARATOR: char = '/';

  /// Determines whether the character is a level separator.
  ///
  /// # Examples
  ///
  /// ```
  /// use hass_mqtt_proto::Topic;
  ///
  /// assert!(Topic::is_separator('/'));
  /// assert!(!Topic::is_separator('❤'));
  /// ```
  #[must_use]
  pub const fn is_separator(c: char) -> bool {
    c == Topic::SEPARATOR
  }
}

////////////////////////////////////////////////////////////////////////////////
// Misc helpers
////////////////////////////////////////////////////////////////////////////////

// Iterate through `iter` while it matches `prefix`; return `None` if `prefix`
// is not a prefix of `iter`, otherwise return `Some(iter_after_prefix)` giving
// `iter` after having exhausted `prefix`.
fn iter_after<'a, 'b, I, J>(mut iter: I, mut prefix: J) -> Option<I>
where
  I: Iterator<Item = &'a str> + Clone,
  J: Iterator<Item = &'b str>,
{
  loop {
    let mut iter_next = iter.clone();
    match (iter_next.next(), prefix.next()) {
      (Some(ref x), Some(ref y)) if x == y => (),
      (Some(_), Some(_)) => return None,
      (Some(_), None) => return Some(iter),
      (None, None) => return Some(iter),
      (None, Some(_)) => return None,
    }
    iter = iter_next;
  }
}

#[inline]
fn is_sep_byte(b: u8) -> bool {
  b == b'/'
}

////////////////////////////////////////////////////////////////////////////////
// The core iterators
////////////////////////////////////////////////////////////////////////////////

/// Component parsing works by a double-ended state machine; the cursors at the
/// front and back of the path each keep track of what parts of the topic have
/// been consumed so far.
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
enum State {
  Body = 0, // foo/bar/baz
  Done = 1,
}

/// An iterator over the levelss of a [`Topic`].
///
/// This `struct` is created by the [`levels`] method on [`Topic`].
/// See its documentation for more.
///
/// # Examples
///
/// ```
/// use hass_mqtt_proto::Topic;
///
/// let topic = Topic::new("tmp/foo/bar.txt");
///
/// for level in topic.levels() {
///   println!("{}", topic);
/// }
/// ```
///
/// [`levels`]: Topic::levels
#[derive(Clone)]
pub struct Levels<'a> {
  // The path left to parse components from
  topic: &'a str,

  // The iterator is double-ended, and these two states keep track of what has
  // been produced from either end
  front: State,
  back: State,
}

impl fmt::Debug for Levels<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    struct DebugHelper<'a>(&'a Topic);

    impl fmt::Debug for DebugHelper<'_> {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.0.levels()).finish()
      }
    }

    f.debug_tuple("Components")
      .field(&DebugHelper(self.as_topic()))
      .finish()
  }
}

impl<'a> Levels<'a> {
  // is the iteration complete?
  #[inline]
  fn finished(&self) -> bool {
    self.front == State::Done || self.back == State::Done || self.front > self.back
  }

  /// Extracts a slice corresponding to the portion of the path remaining for iteration.
  ///
  /// # Examples
  ///
  /// ```
  /// use hass_mqtt_proto::Topic;
  ///
  /// let mut components = Topic::new("tmp/foo/bar.txt").levels();
  /// components.next();
  ///
  /// assert_eq!(Topic::new("foo/bar.txt"), components.as_topic());
  /// ```
  #[must_use]
  pub fn as_topic(&self) -> &'a Topic {
    // let mut comps = self.clone();

    Topic::new(self.topic)
  }

  // parse a component from the left, saying how many bytes to consume to
  // remove the component
  fn parse_next_component(&self) -> (usize, Option<&'a str>) {
    debug_assert!(self.front == State::Body);
    let (extra, comp) = match self.topic.bytes().position(is_sep_byte) {
      None => (0, self.topic),
      Some(i) => (1, &self.topic[..i]),
    };
    (comp.len() + extra, Some(comp))
  }

  // parse a component from the right, saying how many bytes to consume to
  // remove the component
  fn parse_next_component_back(&self) -> (usize, Option<&'a str>) {
    debug_assert!(self.back == State::Body);
    let (extra, comp) = match self.topic.bytes().rposition(is_sep_byte) {
      None => (0, self.topic),
      Some(i) => (1, &self.topic[i + 1..]),
    };
    (comp.len() + extra, Some(comp))
  }
}

impl AsRef<Topic> for Levels<'_> {
  #[inline]
  fn as_ref(&self) -> &Topic {
    self.as_topic()
  }
}

impl AsRef<str> for Levels<'_> {
  #[inline]
  fn as_ref(&self) -> &str {
    self.as_topic().as_str()
  }
}

impl<'a> Iterator for Levels<'a> {
  type Item = &'a str;

  fn next(&mut self) -> Option<Self::Item> {
    while !self.finished() {
      match self.front {
        State::Body if !self.topic.is_empty() => {
          let (size, comp) = self.parse_next_component();
          self.topic = &self.topic[size..];
          if comp.is_some() {
            return comp;
          }
        }
        State::Body => {
          self.front = State::Done;
        }
        State::Done => unreachable!(),
      }
    }

    None
  }
}

impl<'a> DoubleEndedIterator for Levels<'a> {
  fn next_back(&mut self) -> Option<Self::Item> {
    while !self.finished() {
      match self.back {
        State::Body if !self.topic.is_empty() => {
          let (size, comp) = self.parse_next_component_back();
          self.topic = &self.topic[..self.topic.len() - size];
          if comp.is_some() {
            return comp;
          }
        }
        State::Body => {
          self.back = State::Done;
        }
        State::Done => unreachable!(),
      }
    }

    None
  }
}

impl<'a> FusedIterator for Levels<'a> {}

impl<'a> PartialEq for Levels<'a> {
  #[inline]
  fn eq(&self, other: &Levels<'a>) -> bool {
    Iterator::eq(self.clone().rev(), other.clone().rev())
  }
}

impl<'a> Eq for Levels<'a> {}

impl<'a> cmp::PartialOrd for Levels<'a> {
  #[inline]
  fn partial_cmp(&self, other: &Levels<'a>) -> Option<cmp::Ordering> {
    Some(cmp::Ord::cmp(self, other))
  }
}

impl cmp::Ord for Levels<'_> {
  #[inline]
  fn cmp(&self, other: &Self) -> cmp::Ordering {
    compare_levels(self.clone(), other.clone())
  }
}

fn compare_levels(mut left: Levels<'_>, mut right: Levels<'_>) -> cmp::Ordering {
  // Fast path for long shared prefixes
  //
  // - compare raw bytes to find first mismatch
  // - backtrack to find separator before mismatch
  // - if found update state to only do a component-wise comparison on the remainder,
  //   otherwise do it on the full path
  //
  // The fast path isn't taken for paths with a PrefixComponent to avoid backtracking into
  // the middle of one
  if left.front == right.front {
    // this might benefit from a [u8]::first_mismatch simd implementation, if it existed
    let first_difference = match left
      .topic
      .bytes()
      .zip(right.topic.bytes())
      .position(|(a, b)| a != b)
    {
      None if left.topic.len() == right.topic.len() => return cmp::Ordering::Equal,
      None => left.topic.len().min(right.topic.len()),
      Some(diff) => diff,
    };

    if let Some(previous_sep) = left.topic[..first_difference]
      .bytes()
      .rposition(is_sep_byte)
    {
      let mismatched_component_start = previous_sep + 1;
      left.topic = &left.topic[mismatched_component_start..];
      left.front = State::Body;
      right.topic = &right.topic[mismatched_component_start..];
      right.front = State::Body;
    }
  }

  Iterator::cmp(left, right)
}

/// An iterator over [`Topic`] and its ancestors.
///
/// This `struct` is created by the [`ancestors`] method on [`Topic`].
/// See its documentation for more.
///
/// # Examples
///
/// ```
/// use hass_mqtt_proto::Topic;
///
/// let path = Topic::new("/foo/bar");
///
/// for ancestor in path.ancestors() {
///   println!("{}", ancestor);
/// }
/// ```
///
/// [`ancestors`]: Topic::ancestors
#[derive(Copy, Clone, Debug)]
pub struct Ancestors<'a> {
  next: Option<&'a Topic>,
}

impl<'a> Iterator for Ancestors<'a> {
  type Item = &'a Topic;

  #[inline]
  fn next(&mut self) -> Option<Self::Item> {
    let next = self.next;
    self.next = next.and_then(Topic::parent);
    next
  }
}

impl FusedIterator for Ancestors<'_> {}

////////////////////////////////////////////////////////////////////////////////
// Basic types and traits
////////////////////////////////////////////////////////////////////////////////

/// An owned, mutable topic (akin to [`String`]).
///
/// This type provides methods like [`push`] that mutate the topic in place.
/// It also implements [`Deref`] to [`Topic`], meaning that all methods on [`Topic`]
/// slices are available on `TopicBuf` values as well.
///
/// [`push`]: TopicBuf::push
///
/// # Examples
///
/// You can use [`push`] to build up a `TopicBuf` from
/// levels:
///
/// ```
/// use hass_mqtt_proto::TopicBuf;
///
/// let mut topic = TopicBuf::new();
///
/// topic.push("foo");
/// topic.push("bar");
/// ```
///
/// However, [`push`] is best used for dynamic situations. This is a better way
/// to do this when you know all of the levels ahead of time:
///
/// ```
/// use hass_mqtt_proto::TopicBuf;
///
/// let topic: TopicBuf = ["foo", "bar"].iter().collect();
/// ```
///
/// We can still do better than this! Since these are all strings, we can use
/// `From::from`:
///
/// ```
/// use hass_mqtt_proto::TopicBuf;
///
/// let topic = TopicBuf::from("foo/bar");
/// ```
///
/// Which method works best depends on what kind of situation you're in.
pub struct TopicBuf {
  inner: String,
}

impl TopicBuf {
  /// Allocates an empty `TopicBuf`.
  ///
  /// # Examples
  ///
  /// ```
  /// use hass_mqtt_proto::TopicBuf;
  ///
  /// let topic = TopicBuf::new();
  /// ```
  #[inline]
  pub const fn new() -> Self {
    TopicBuf {
      inner: String::new(),
    }
  }

  /// Creates a new `TopicBuf` with a given capacity used to create the
  /// internal [`String`]. See [`with_capacity`] defined on [`String`].
  ///
  /// # Examples
  ///
  /// ```
  /// use hass_mqtt_proto::TopicBuf;
  ///
  /// let mut topic = TopicBuf::with_capacity(10);
  /// let capacity = topic.capacity();
  ///
  /// // This push is done without reallocating
  /// topic.push("foo");
  ///
  /// assert_eq!(capacity, topic.capacity());
  /// ```
  ///
  /// [`with_capacity`]: String::with_capacity
  #[inline]
  pub fn with_capacity(capacity: usize) -> Self {
    TopicBuf {
      inner: String::with_capacity(capacity),
    }
  }

  /// Coerces to a [`Topic`] slice.
  ///
  /// # Examples
  ///
  /// ```
  /// use hass_mqtt_proto::{Topic, TopicBuf};
  ///
  /// let t = TopicBuf::from("/test");
  /// assert_eq!(Topic::new("/test"), t.as_topic());
  /// ```
  #[inline]
  pub fn as_topic(&self) -> &Topic {
    self
  }

  /// Extends `self` with `path`.
  ///
  /// # Examples
  ///
  /// Pushing a path extends the existing path:
  ///
  /// ```
  /// use hass_mqtt_proto::TopicBuf;
  ///
  /// let mut path = TopicBuf::from("tmp");
  /// path.push("file");
  /// assert_eq!(path, TopicBuf::from("tmp/file.bk"));
  /// ```
  pub fn push<T: AsRef<Topic>>(&mut self, path: T) {
    self._push(path.as_ref())
  }

  fn _push(&mut self, topic: &Topic) {
    self.inner.reserve(topic.as_str().len() + 1);
    self.inner.push(Topic::SEPARATOR);
    self.inner.push_str(topic.as_str());
  }

  /// Truncates `self` to [`self.parent`].
  ///
  /// Returns `false` and does nothing if [`self.parent`] is [`None`].
  /// Otherwise, returns `true`.
  ///
  /// [`self.parent`]: Topic::parent
  ///
  /// # Examples
  ///
  /// ```
  /// use hass_mqtt_proto::{Topic, TopicBuf};
  ///
  /// let mut p = TopicBuf::from("spirited/away.rs");
  ///
  /// p.pop();
  /// assert_eq!(Topic::new("/spirited"), p);
  /// p.pop();
  /// assert_eq!(Topic::new("/"), p);
  /// ```
  pub fn pop(&mut self) -> bool {
    match self.parent().map(|p| p.inner.len()) {
      Some(len) => {
        self.inner.truncate(len);
        true
      }
      None => false,
    }
  }

  /// Consumes the `TopicBuf`, yielding its internal [`String`] storage.
  ///
  /// # Examples
  ///
  /// ```
  /// use hass_mqtt_proto::TopicBuf;
  ///
  /// let p = TopicBuf::from("/the/head");
  /// let os_str = p.into_string();
  /// ```
  #[inline]
  pub fn into_string(self) -> String {
    self.inner
  }

  /// Converts this `TopicBuf` into a [boxed](Box) [`Topic`].
  #[inline]
  pub fn into_boxed_topic(self) -> Box<Topic> {
    let rw = Box::into_raw(self.inner.into_boxed_str()) as *mut Topic;
    unsafe { Box::from_raw(rw) }
  }

  /// Invokes [`capacity`] on the underlying instance of [`String`].
  ///
  /// [`capacity`]: String::capacity
  #[inline]
  pub fn capacity(&self) -> usize {
    self.inner.capacity()
  }

  /// Invokes [`clear`] on the underlying instance of [`String`].
  ///
  /// [`clear`]: String::clear
  #[inline]
  pub fn clear(&mut self) {
    self.inner.clear()
  }

  /// Invokes [`reserve`] on the underlying instance of [`String`].
  ///
  /// [`reserve`]: String::reserve
  #[inline]
  pub fn reserve(&mut self, additional: usize) {
    self.inner.reserve(additional)
  }

  /// Invokes [`reserve_exact`] on the underlying instance of [`String`].
  ///
  /// [`reserve_exact`]: String::reserve_exact
  #[inline]
  pub fn reserve_exact(&mut self, additional: usize) {
    self.inner.reserve_exact(additional)
  }

  /// Invokes [`shrink_to_fit`] on the underlying instance of [`String`].
  ///
  /// [`shrink_to_fit`]: String::shrink_to_fit
  #[inline]
  pub fn shrink_to_fit(&mut self) {
    self.inner.shrink_to_fit()
  }

  /// Invokes [`shrink_to`] on the underlying instance of [`String`].
  ///
  /// [`shrink_to`]: String::shrink_to
  #[inline]
  pub fn shrink_to(&mut self, min_capacity: usize) {
    self.inner.shrink_to(min_capacity)
  }
}

impl Clone for TopicBuf {
  #[inline]
  fn clone(&self) -> Self {
    TopicBuf {
      inner: self.inner.clone(),
    }
  }

  #[inline]
  fn clone_from(&mut self, source: &Self) {
    self.inner.clone_from(&source.inner)
  }
}

impl From<&Topic> for Box<Topic> {
  /// Creates a boxed [`Topic`] from a reference.
  ///
  /// This will allocate and clone `path` to it.
  fn from(path: &Topic) -> Box<Topic> {
    let boxed: Box<str> = path.inner.into();
    let rw = Box::into_raw(boxed) as *mut Topic;
    unsafe { Box::from_raw(rw) }
  }
}

impl From<Cow<'_, Topic>> for Box<Topic> {
  /// Creates a boxed [`Topic`] from a clone-on-write pointer.
  ///
  /// Converting from a `Cow::Owned` does not clone or allocate.
  #[inline]
  fn from(cow: Cow<'_, Topic>) -> Box<Topic> {
    match cow {
      Cow::Borrowed(path) => Box::from(path),
      Cow::Owned(path) => Box::from(path),
    }
  }
}

impl From<Box<Topic>> for TopicBuf {
  /// Converts a `Box<Topic>` into a `TopicBuf`
  ///
  /// This conversion does not allocate or copy memory.
  #[inline]
  fn from(boxed: Box<Topic>) -> TopicBuf {
    boxed.into_topic_buf()
  }
}

impl From<TopicBuf> for Box<Topic> {
  /// Converts a `TopicBuf` into a `Box<Topic>`
  ///
  /// This conversion currently should not allocate memory,
  /// but this behavior is not guaranteed on all platforms or in all future versions.
  #[inline]
  fn from(p: TopicBuf) -> Box<Topic> {
    p.into_boxed_topic()
  }
}

impl Clone for Box<Topic> {
  #[inline]
  fn clone(&self) -> Self {
    self.to_topic_buf().into_boxed_topic()
  }
}

impl<T: ?Sized + AsRef<str>> From<&T> for TopicBuf {
  /// Converts a borrowed `str` to a `TopicBuf`.
  ///
  /// Allocates a [`TopicBuf`] and copies the data into it.
  #[inline]
  fn from(s: &T) -> TopicBuf {
    TopicBuf::from(s.as_ref().to_string())
  }
}

impl From<String> for TopicBuf {
  /// Converts an [`String`] into a [`TopicBuf`]
  ///
  /// This conversion does not allocate or copy memory.
  #[inline]
  fn from(s: String) -> TopicBuf {
    TopicBuf { inner: s }
  }
}

impl From<TopicBuf> for String {
  /// Converts a [`TopicBuf`] into an [`String`]
  ///
  /// This conversion does not allocate or copy memory.
  #[inline]
  fn from(buf: TopicBuf) -> String {
    buf.inner
  }
}

impl FromStr for TopicBuf {
  type Err = core::convert::Infallible;

  #[inline]
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    Ok(TopicBuf::from(s))
  }
}

impl<P: AsRef<Topic>> iter::FromIterator<P> for TopicBuf {
  fn from_iter<I: IntoIterator<Item = P>>(iter: I) -> TopicBuf {
    let mut buf = TopicBuf::new();
    buf.extend(iter);
    buf
  }
}

impl<P: AsRef<Topic>> iter::Extend<P> for TopicBuf {
  fn extend<I: IntoIterator<Item = P>>(&mut self, iter: I) {
    iter.into_iter().for_each(move |p| self.push(p.as_ref()));
  }

  // #[inline]
  // fn extend_one(&mut self, p: P) {
  //   self.push(p.as_ref());
  // }
}

impl fmt::Debug for TopicBuf {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Debug::fmt(&**self, formatter)
  }
}

impl ops::Deref for TopicBuf {
  type Target = Topic;

  #[inline]
  fn deref(&self) -> &Topic {
    Topic::new(&self.inner)
  }
}

impl Borrow<Topic> for TopicBuf {
  #[inline]
  fn borrow(&self) -> &Topic {
    ops::Deref::deref(self)
  }
}

impl Default for TopicBuf {
  #[inline]
  fn default() -> Self {
    TopicBuf::new()
  }
}

impl<'a> From<&'a Topic> for Cow<'a, Topic> {
  /// Creates a clone-on-write pointer from a reference to
  /// [`Topic`].
  ///
  /// This conversion does not clone or allocate.
  #[inline]
  fn from(s: &'a Topic) -> Cow<'a, Topic> {
    Cow::Borrowed(s)
  }
}

impl<'a> From<TopicBuf> for Cow<'a, Topic> {
  /// Creates a clone-on-write pointer from an owned
  /// instance of [`TopicBuf`].
  ///
  /// This conversion does not clone or allocate.
  #[inline]
  fn from(s: TopicBuf) -> Cow<'a, Topic> {
    Cow::Owned(s)
  }
}

impl<'a> From<&'a TopicBuf> for Cow<'a, Topic> {
  /// Creates a clone-on-write pointer from a reference to
  /// [`TopicBuf`].
  ///
  /// This conversion does not clone or allocate.
  #[inline]
  fn from(p: &'a TopicBuf) -> Cow<'a, Topic> {
    Cow::Borrowed(p.as_topic())
  }
}

impl<'a> From<Cow<'a, Topic>> for TopicBuf {
  /// Converts a clone-on-write pointer to an owned topic.
  ///
  /// Converting from a `Cow::Owned` does not clone or allocate.
  #[inline]
  fn from(p: Cow<'a, Topic>) -> Self {
    p.into_owned()
  }
}

impl From<TopicBuf> for Arc<Topic> {
  /// Converts a [`TopicBuf`] into an [`Arc`] by moving the [`TopicBuf`] data into a new [`Arc`] buffer.

  #[inline]
  fn from(s: TopicBuf) -> Arc<Topic> {
    let arc: Arc<str> = Arc::from(s.into_string());
    unsafe { Arc::from_raw(Arc::into_raw(arc) as *const Topic) }
  }
}

impl From<&Topic> for Arc<Topic> {
  /// Converts a [`Topic`] into an [`Arc`] by copying the [`Topic`] data into a new [`Arc`] buffer.

  #[inline]
  fn from(s: &Topic) -> Arc<Topic> {
    let arc: Arc<str> = Arc::from(s.as_str());
    unsafe { Arc::from_raw(Arc::into_raw(arc) as *const Topic) }
  }
}

impl From<TopicBuf> for Rc<Topic> {
  /// Converts a [`TopicBuf`] into an [`Rc`] by moving the [`TopicBuf`] data into a new `Rc` buffer.

  #[inline]
  fn from(s: TopicBuf) -> Rc<Topic> {
    let rc: Rc<str> = Rc::from(s.into_string());
    unsafe { Rc::from_raw(Rc::into_raw(rc) as *const Topic) }
  }
}

impl From<&Topic> for Rc<Topic> {
  /// Converts a [`Topic`] into an [`Rc`] by copying the [`Topic`] data into a new `Rc` buffer.

  #[inline]
  fn from(s: &Topic) -> Rc<Topic> {
    let rc: Rc<str> = Rc::from(s.as_str());
    unsafe { Rc::from_raw(Rc::into_raw(rc) as *const Topic) }
  }
}

impl ToOwned for Topic {
  type Owned = TopicBuf;

  #[inline]
  fn to_owned(&self) -> TopicBuf {
    self.to_topic_buf()
  }

  // #[inline]
  // fn clone_into(&self, target: &mut TopicBuf) {
  //   self.inner.clone_into(&mut target.inner);
  // }
}

impl cmp::PartialEq for TopicBuf {
  #[inline]
  fn eq(&self, other: &TopicBuf) -> bool {
    self.as_topic() == other.as_topic()
  }
}

impl Hash for TopicBuf {
  fn hash<H: Hasher>(&self, h: &mut H) {
    self.as_topic().hash(h)
  }
}

impl cmp::Eq for TopicBuf {}

impl cmp::PartialOrd for TopicBuf {
  #[inline]
  fn partial_cmp(&self, other: &TopicBuf) -> Option<cmp::Ordering> {
    self.as_topic().partial_cmp(other.as_topic())
  }
}

impl cmp::Ord for TopicBuf {
  #[inline]
  fn cmp(&self, other: &TopicBuf) -> cmp::Ordering {
    self.as_topic().cmp(other.as_topic())
  }
}

impl AsRef<str> for TopicBuf {
  #[inline]
  fn as_ref(&self) -> &str {
    &self.inner[..]
  }
}

/// A slice of a path (akin to [`str`]).
///
/// This type supports a number of operations for inspecting a topic, including
/// breaking the path into its levels (separated by `/`), and so on.
///
/// This is an *unsized* type, meaning that it must always be used behind a
/// pointer like `&` or [`Box`]. For an owned version of this type,
/// see [`TopicBuf`].
///
/// # Examples
///
/// ```
/// use hass_mqtt_proto::Topic;
///
/// // Note: this example does work on Windows
/// let topic = Topic::new("foo/bar.txt");
///
/// let parent = topic.parent();
/// assert_eq!(parent, Some(Topic::new("foo")));
/// ```
#[repr(transparent)]
pub struct Topic {
  inner: str,
}

/// An error returned from [`Topic::strip_prefix`] if the prefix was not found.
///
/// This `struct` is created by the [`strip_prefix`] method on [`Topic`].
/// See its documentation for more.
///
/// [`strip_prefix`]: Topic::strip_prefix
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StripPrefixError(());

impl Topic {
  /// Directly wraps a string slice as a `Topic` slice.
  ///
  /// This is a cost-free conversion.
  ///
  /// # Examples
  ///
  /// ```
  /// use hass_mqtt_proto::Topic;
  ///
  /// Topic::new("foo");
  /// ```
  ///
  /// You can create `Topic`s from `String`s, or even other `Topic`s:
  ///
  /// ```
  /// use hass_mqtt_proto::Topic;
  ///
  /// let string = String::from("foo.txt");
  /// let from_string = Topic::new(&string);
  /// let from_topic = Topic::new(&from_string);
  /// assert_eq!(from_string, from_topic);
  /// ```
  pub fn new<S: AsRef<str> + ?Sized>(s: &S) -> &Topic {
    unsafe { &*(s.as_ref() as *const str as *const Topic) }
  }

  /// Yields the underlying [`str`] slice.
  ///
  /// # Examples
  ///
  /// ```
  /// use hass_mqtt_proto::Topic;
  ///
  /// let str = Topic::new("foo.txt").as_str();
  /// assert_eq!(str, "foo.txt");
  /// ```
  #[inline]
  pub fn as_str(&self) -> &str {
    &self.inner
  }

  /// Converts a `Topic` to an owned [`TopicBuf`].
  ///
  /// # Examples
  ///
  /// ```
  /// use hass_mqtt_proto::Topic;
  ///
  /// let topic_buf = Topic::new("foo.txt").to_topic_buf();
  /// assert_eq!(topic_buf, hass_mqtt_proto::TopicBuf::from("foo.txt"));
  /// ```
  pub fn to_topic_buf(&self) -> TopicBuf {
    TopicBuf::from(self.inner.to_string())
  }

  /// Returns the `Topic` without its final component, if there is one.
  ///
  /// Returns [`None`] if the path terminates in a root or prefix.
  ///
  /// # Examples
  ///
  /// ```
  /// use hass_mqtt_proto::Topic;
  ///
  /// let path = Topic::new("foo/bar/baz");
  /// let parent = path.parent().unwrap();
  /// assert_eq!(parent, Topic::new("foo/bar"));
  ///
  /// let grand_parent = parent.parent().unwrap();
  /// assert_eq!(grand_parent, Topic::new("foo"));
  /// assert_eq!(grand_parent.parent(), None);
  /// ```
  pub fn parent(&self) -> Option<&Topic> {
    let mut levels = self.levels();
    let level = levels.next_back();
    level.map(|_| levels.as_topic())
  }

  /// Produces an iterator over `Topic` and its ancestors.
  ///
  /// The iterator will yield the `Topic` that is returned if the [`parent`] method is used zero
  /// or more times. That means, the iterator will yield `&self`, `&self.parent().unwrap()`,
  /// `&self.parent().unwrap().parent().unwrap()` and so on. If the [`parent`] method returns
  /// [`None`], the iterator will do likewise. The iterator will always yield at least one value,
  /// namely `&self`.
  ///
  /// # Examples
  ///
  /// ```
  /// use hass_mqtt_proto::Topic;
  ///
  /// let mut ancestors = Topic::new("/foo/bar").ancestors();
  /// assert_eq!(ancestors.next(), Some(Topic::new("/foo/bar")));
  /// assert_eq!(ancestors.next(), Some(Topic::new("/foo")));
  /// assert_eq!(ancestors.next(), Some(Topic::new("/")));
  /// assert_eq!(ancestors.next(), None);
  /// ```
  ///
  /// [`parent`]: Topic::parent
  #[inline]
  pub fn ancestors(&self) -> Ancestors<'_> {
    Ancestors { next: Some(self) }
  }

  /// Returns a topic that, when joined onto `base`, yields `self`.
  ///
  /// # Errors
  ///
  /// If `base` is not a prefix of `self` (i.e., [`starts_with`]
  /// returns `false`), returns [`Err`].
  ///
  /// [`starts_with`]: Topic::starts_with
  ///
  /// # Examples
  ///
  /// ```
  /// use hass_mqtt_proto::{Topic, TopicBuf};
  ///
  /// let path = Topic::new("test/haha/foo.txt");
  ///
  /// assert_eq!(path.strip_prefix("test"), Ok(Topic::new("haha/foo.txt")));
  /// assert_eq!(path.strip_prefix("test/"), Ok(Topic::new("haha/foo.txt")));
  /// assert_eq!(path.strip_prefix("test/haha/foo.txt"), Ok(Topic::new("")));
  ///
  /// assert!(path.strip_prefix("test/haha/foo.txt/").is_err());
  /// assert!(path.strip_prefix("/test").is_err());
  /// assert!(path.strip_prefix("haha").is_err());
  ///
  /// let prefix = TopicBuf::from("test");
  /// assert_eq!(path.strip_prefix(prefix), Ok(Topic::new("haha/foo.txt")));
  /// ```
  pub fn strip_prefix<P>(&self, base: P) -> Result<&Topic, StripPrefixError>
  where
    P: AsRef<Topic>,
  {
    self._strip_prefix(base.as_ref())
  }

  fn _strip_prefix(&self, base: &Topic) -> Result<&Topic, StripPrefixError> {
    iter_after(self.levels(), base.levels())
      .map(|c| c.as_topic())
      .ok_or(StripPrefixError(()))
  }

  /// Determines whether `base` is a prefix of `self`.
  ///
  /// Only considers whole path levels to match.
  ///
  /// # Examples
  ///
  /// ```
  /// use hass_mqtt_proto::Topic;
  ///
  /// let path = Topic::new("etc/passwd");
  ///
  /// assert!(path.starts_with("etc"));
  /// assert!(path.starts_with("etc/"));
  /// assert!(path.starts_with("etc/passwd"));
  ///
  /// assert!(!path.starts_with("etc/passwd/")); // extra slash is not okay
  /// assert!(!path.starts_with("etc/passwd///")); // multiple extra slashes are not okay
  /// assert!(!path.starts_with("e"));
  /// assert!(!path.starts_with("etc/passwd.txt"));
  ///
  /// assert!(!Topic::new("etc/foo.rs").starts_with("etc/foo"));
  /// ```
  pub fn starts_with<P: AsRef<Topic>>(&self, base: P) -> bool {
    self._starts_with(base.as_ref())
  }

  fn _starts_with(&self, base: &Topic) -> bool {
    iter_after(self.levels(), base.levels()).is_some()
  }

  /// Determines whether `child` is a suffix of `self`.
  ///
  /// Only considers whole path levels to match.
  ///
  /// # Examples
  ///
  /// ```
  /// use hass_mqtt_proto::Topic;
  ///
  /// let path = Topic::new("etc/resolv.conf");
  ///
  /// assert!(path.ends_with("resolv.conf"));
  /// assert!(path.ends_with("etc/resolv.conf"));
  ///
  /// assert!(!path.ends_with("/etc/resolv.conf"));
  /// assert!(!path.ends_with("/resolv.conf"));
  /// assert!(!path.ends_with("conf")); // must match entire segment
  /// ```
  pub fn ends_with<P: AsRef<Topic>>(&self, child: P) -> bool {
    self._ends_with(child.as_ref())
  }

  fn _ends_with(&self, child: &Topic) -> bool {
    iter_after(self.levels().rev(), child.levels().rev()).is_some()
  }

  /// Creates an owned [`TopicBuf`] with `path` adjoined to `self`.
  ///
  /// See [`TopicBuf::push`] for more details on what it means to adjoin a path.
  ///
  /// # Examples
  ///
  /// ```
  /// use hass_mqtt_proto::{Topic, TopicBuf};
  ///
  /// assert_eq!(Topic::new("etc").join("passwd"), TopicBuf::from("etc/passwd"));
  /// ```
  #[must_use]
  pub fn join<P: AsRef<Topic>>(&self, path: P) -> TopicBuf {
    self._join(path.as_ref())
  }

  fn _join(&self, path: &Topic) -> TopicBuf {
    let mut buf = self.to_topic_buf();
    buf.push(path);
    buf
  }

  /// Produces an iterator over the levels of the topic.
  ///
  /// # Examples
  ///
  /// ```
  /// use hass_mqtt_proto::Topic;
  ///
  /// let mut levels = Topic::new("tmp/foo.txt").levels();
  ///
  /// assert_eq!(levels.next(), Some("tmp"));
  /// assert_eq!(levels.next(), Some("foo.txt"));
  /// assert_eq!(levels.next(), None);
  /// ```
  pub fn levels(&self) -> Levels<'_> {
    Levels {
      topic: &self.inner,
      front: State::Body,
      back: State::Body,
    }
  }

  /// Converts a [`Box<Topic>`](Box) into a [`TopicBuf`] without copying or
  /// allocating.
  pub fn into_topic_buf(self: Box<Topic>) -> TopicBuf {
    let rw = Box::into_raw(self) as *mut str;
    let inner = unsafe { Box::from_raw(rw) };
    TopicBuf {
      inner: String::from(inner),
    }
  }
}

impl AsRef<str> for Topic {
  #[inline]
  fn as_ref(&self) -> &str {
    &self.inner
  }
}

impl fmt::Debug for Topic {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Debug::fmt(&self.inner, formatter)
  }
}

impl fmt::Display for Topic {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Display::fmt(&self.inner, formatter)
  }
}

impl cmp::PartialEq for Topic {
  #[inline]
  fn eq(&self, other: &Topic) -> bool {
    self.levels() == other.levels()
  }
}

impl Hash for Topic {
  fn hash<H: Hasher>(&self, h: &mut H) {
    for level in self.levels() {
      level.hash(h);
    }
  }
}

impl cmp::Eq for Topic {}

impl cmp::PartialOrd for Topic {
  #[inline]
  fn partial_cmp(&self, other: &Topic) -> Option<cmp::Ordering> {
    Some(cmp::Ord::cmp(self, other))
  }
}

impl cmp::Ord for Topic {
  #[inline]
  fn cmp(&self, other: &Topic) -> cmp::Ordering {
    compare_levels(self.levels(), other.levels())
  }
}

impl AsRef<Topic> for Topic {
  #[inline]
  fn as_ref(&self) -> &Topic {
    self
  }
}

impl AsRef<Topic> for str {
  #[inline]
  fn as_ref(&self) -> &Topic {
    Topic::new(self)
  }
}

impl AsRef<Topic> for Cow<'_, str> {
  #[inline]
  fn as_ref(&self) -> &Topic {
    Topic::new(self)
  }
}

impl AsRef<Topic> for String {
  #[inline]
  fn as_ref(&self) -> &Topic {
    Topic::new(self)
  }
}

impl AsRef<Topic> for TopicBuf {
  #[inline]
  fn as_ref(&self) -> &Topic {
    self
  }
}

// impl<'a> IntoIterator for &'a TopicBuf {
//   type Item = &'a str;
//   type IntoIter = Iter<'a>;

//   #[inline]
//   fn into_iter(self) -> Iter<'a> {
//     self.iter()
//   }
// }

// impl<'a> IntoIterator for &'a Topic {
//   type Item = &'a str;
//   type IntoIter = Iter<'a>;

//   #[inline]
//   fn into_iter(self) -> Iter<'a> {
//     self.iter()
//   }
// }

macro_rules! impl_cmp {
  ($lhs:ty, $rhs: ty) => {
    impl<'a, 'b> PartialEq<$rhs> for $lhs {
      #[inline]
      fn eq(&self, other: &$rhs) -> bool {
        <Topic as PartialEq>::eq(self, other)
      }
    }

    impl<'a, 'b> PartialEq<$lhs> for $rhs {
      #[inline]
      fn eq(&self, other: &$lhs) -> bool {
        <Topic as PartialEq>::eq(self, other)
      }
    }

    impl<'a, 'b> PartialOrd<$rhs> for $lhs {
      #[inline]
      fn partial_cmp(&self, other: &$rhs) -> Option<cmp::Ordering> {
        <Topic as PartialOrd>::partial_cmp(self, other)
      }
    }

    impl<'a, 'b> PartialOrd<$lhs> for $rhs {
      #[inline]
      fn partial_cmp(&self, other: &$lhs) -> Option<cmp::Ordering> {
        <Topic as PartialOrd>::partial_cmp(self, other)
      }
    }
  };
}

impl_cmp!(TopicBuf, Topic);
impl_cmp!(TopicBuf, &'a Topic);
impl_cmp!(Cow<'a, Topic>, Topic);
impl_cmp!(Cow<'a, Topic>, &'b Topic);
impl_cmp!(Cow<'a, Topic>, TopicBuf);

macro_rules! impl_cmp_str {
  ($lhs:ty, $rhs: ty) => {
    impl<'a, 'b> PartialEq<$rhs> for $lhs {
      #[inline]
      fn eq(&self, other: &$rhs) -> bool {
        <Topic as PartialEq>::eq(self, other.as_ref())
      }
    }

    impl<'a, 'b> PartialEq<$lhs> for $rhs {
      #[inline]
      fn eq(&self, other: &$lhs) -> bool {
        <Topic as PartialEq>::eq(self.as_ref(), other)
      }
    }

    impl<'a, 'b> PartialOrd<$rhs> for $lhs {
      #[inline]
      fn partial_cmp(&self, other: &$rhs) -> Option<cmp::Ordering> {
        <Topic as PartialOrd>::partial_cmp(self, other.as_ref())
      }
    }

    impl<'a, 'b> PartialOrd<$lhs> for $rhs {
      #[inline]
      fn partial_cmp(&self, other: &$lhs) -> Option<cmp::Ordering> {
        <Topic as PartialOrd>::partial_cmp(self.as_ref(), other)
      }
    }
  };
}

impl_cmp_str!(TopicBuf, str);
impl_cmp_str!(TopicBuf, &'a str);
// impl_cmp_str!(TopicBuf, Cow<'a, str>);
impl_cmp_str!(TopicBuf, String);
impl_cmp_str!(Topic, str);
impl_cmp_str!(Topic, &'a str);
impl_cmp_str!(Topic, Cow<'a, str>);
impl_cmp_str!(Topic, String);
impl_cmp_str!(&'a Topic, str);
// impl_cmp_str!(&'a Topic, Cow<'b, str>);
impl_cmp_str!(&'a Topic, String);
// impl_cmp_str!(Cow<'a, Topic>, str);
// impl_cmp_str!(Cow<'a, Topic>, &'b str);
// impl_cmp_str!(Cow<'a, Topic>, String);

impl fmt::Display for StripPrefixError {
  #[allow(deprecated, deprecated_in_future)]
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.description().fmt(f)
  }
}

impl Error for StripPrefixError {
  fn description(&self) -> &str {
    "prefix not found"
  }
}
