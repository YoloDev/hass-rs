// Taken from tokio_stream: https://github.com/tokio-rs/tokio/blob/22cff80048c62ed0fa20065888667d00d5aedd14/tokio-stream/src/stream_map.rs

use std::{
	cell::Cell,
	collections::hash_map::RandomState,
	hash::BuildHasher,
	sync::atomic::{AtomicU32, Ordering::Relaxed},
};

/// Fast random number generate
///
/// Implement xorshift64+: 2 32-bit xorshift sequences added together.
/// Shift triplet `[17,7,16]` was calculated as indicated in Marsaglia's
/// Xorshift paper: <https://www.jstatsoft.org/article/view/v008i14/xorshift.pdf>
/// This generator passes the SmallCrush suite, part of TestU01 framework:
/// <http://simul.iro.umontreal.ca/testu01/tu01.html>
#[derive(Debug)]
pub(crate) struct FastRand {
	one: Cell<u32>,
	two: Cell<u32>,
}

impl Default for FastRand {
	fn default() -> Self {
		static COUNTER: AtomicU32 = AtomicU32::new(1);

		let rand_state = RandomState::new();

		// Get the seed
		let seed = rand_state.hash_one(COUNTER.fetch_add(1, Relaxed));

		Self::new(seed)
	}
}

impl FastRand {
	/// Initialize a new, thread-local, fast random number generator.
	pub(crate) fn new(seed: u64) -> FastRand {
		let one = (seed >> 32) as u32;
		let mut two = seed as u32;

		if two == 0 {
			// This value cannot be zero
			two = 1;
		}

		FastRand {
			one: Cell::new(one),
			two: Cell::new(two),
		}
	}

	pub(crate) fn fastrand_n(&self, n: u32) -> u32 {
		// This is similar to fastrand() % n, but faster.
		// See https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction/
		let mul = (self.fastrand() as u64).wrapping_mul(n as u64);
		(mul >> 32) as u32
	}

	fn fastrand(&self) -> u32 {
		let mut s1 = self.one.get();
		let s0 = self.two.get();

		s1 ^= s1 << 17;
		s1 = s1 ^ s0 ^ s1 >> 7 ^ s0 >> 16;

		self.one.set(s0);
		self.two.set(s1);

		s0.wrapping_add(s1)
	}
}
