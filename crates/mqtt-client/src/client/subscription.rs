mod rand;

use futures::FutureExt;
use std::{
	future::Future,
	pin::Pin,
	sync::Arc,
	task::{Context, Poll},
};
use tokio::sync::oneshot;

type RouteId = generational_arena::Index;

#[derive(Clone)]
pub(crate) struct SubscriptionToken {
	_id: RouteId,
	#[allow(unused)]
	lifetime: Arc<oneshot::Sender<()>>,
}

#[derive(Debug)]
struct SubscriptionRef {
	id: RouteId,
	lifetime: Box<oneshot::Receiver<()>>,
}

impl Future for SubscriptionRef {
	type Output = RouteId;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let this = self.get_mut();
		let id = this.id;
		this.lifetime.poll_unpin(cx).map(|_| id)
	}
}

#[derive(Default, Debug)]
pub(super) struct Subscriptions {
	rand: rand::FastRand,
	subscriptions: Vec<SubscriptionRef>,
}

static_assertions::assert_impl_all!(Subscriptions: Unpin);

impl Subscriptions {
	pub(super) fn new() -> Self {
		Self::default()
	}

	pub(super) fn insert(&mut self, id: RouteId) -> SubscriptionToken {
		let (lifetime_sender, lifetime_receiver) = oneshot::channel();
		self.subscriptions.push(SubscriptionRef {
			id,
			lifetime: Box::new(lifetime_receiver),
		});

		SubscriptionToken {
			_id: id,
			lifetime: Arc::new(lifetime_sender),
		}
	}

	pub(super) fn dropped(&mut self) -> impl Future<Output = RouteId> + '_ {
		DroppedSubscriptionsStream {
			subscriptions: self,
		}
	}
}

struct DroppedSubscriptionsStream<'a> {
	subscriptions: &'a mut Subscriptions,
}

impl<'a> Future for DroppedSubscriptionsStream<'a> {
	type Output = RouteId;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let this = &mut self.get_mut().subscriptions;
		let start = this.rand.fastrand_n(this.subscriptions.len() as u32) as usize;

		let (snd, fst) = this.subscriptions.split_at_mut(start);
		let iter = fst.iter_mut().chain(snd.iter_mut());
		for subscription in iter {
			if subscription.lifetime.poll_unpin(cx).is_ready() {
				let id = subscription.id;
				let idx = this.subscriptions.iter().position(|s| s.id == id).unwrap();
				this.subscriptions.swap_remove(idx);
				return Poll::Ready(id);
			}
		}

		Poll::Pending
	}
}
