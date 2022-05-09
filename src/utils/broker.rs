// broker is also a actor
// broker is also a service

// broker has message publish implemented
// broker could return a proxy
// broker call will return a spmc channel
// actor an subscribe to brokers after it implement the handler

use std::{marker::PhantomData, sync::atomic::AtomicU64};

use dashmap::DashMap;

use crate::actor::{
    addr::{Addr, WeakAddr},
    context::Context,
    message::{Handler, Message},
    proxy::Proxy,
    runner::Actor,
};

#[async_trait::async_trait]
pub trait Broker<T: Message + Sync + Clone>:
    Handler<Publish<T>> + Handler<Subscribe<T>> + Handler<Unsubscribe>
{
}

pub struct Publish<T: Message + Sync + Clone + Clone>(pub T);

impl<T: Message + Sync + Clone> Message for Publish<T> {
    type Result = ();
}

pub type SubscriptionID = u64;

pub struct Subscribe<T: Message + Sync + Clone> {
    addr: WeakAddr,
    proxy: Proxy<T>,
}
impl<T: Message + Sync + Clone> Subscribe<T> {
    pub async fn from_addr<A: Handler<T>>(addr: Addr) -> Self {
        Self {
            addr: addr.downgrade(),
            proxy: addr.proxy::<A, T>().await,
        }
    }
}

impl<T: Message + Sync + Clone> Message for Subscribe<T> {
    type Result = SubscriptionID;
}

pub struct Unsubscribe(SubscriptionID);
impl Message for Unsubscribe {
    type Result = ();
}

pub struct DefaultBroker<T: Message + Sync + Clone> {
    counter: AtomicU64,
    subscriptions: DashMap<SubscriptionID, Subscribe<T>>,
    _marker: PhantomData<T>,
}

impl<T: Message + Sync + Clone> DefaultBroker<T> {
    pub fn new() -> Self {
        Self {
            counter: AtomicU64::new(0),
            subscriptions: DashMap::new(),
            _marker: PhantomData,
        }
    }
}

impl<T: Message + Sync + Clone> Default for DefaultBroker<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Message + Sync + Clone> Actor for DefaultBroker<T> {}

#[async_trait::async_trait]
impl<T: Message + Sync + Clone> Handler<Subscribe<T>> for DefaultBroker<T> {
    async fn handle(&self, _ctx: &Context, msg: Subscribe<T>) -> anyhow::Result<SubscriptionID> {
        let id = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.subscriptions.insert(id, msg);
        Ok(id)
    }
}
#[async_trait::async_trait]

impl<T: Message + Sync + Clone> Handler<Unsubscribe> for DefaultBroker<T> {
    async fn handle(&self, _ctx: &Context, msg: Unsubscribe) -> anyhow::Result<()> {
        self.subscriptions.remove(&msg.0);
        Ok(())
    }
}

#[async_trait::async_trait]
impl<T: Message + Sync + Clone> Handler<Publish<T>> for DefaultBroker<T> {
    async fn handle(&self, _ctx: &Context, msg: Publish<T>) -> anyhow::Result<()> {
        for kv in self.subscriptions.iter() {
            kv.proxy.call(msg.0.clone()).await?;
        }
        Ok(())
    }
}

impl<T: Message + Sync + Clone> Broker<T> for DefaultBroker<T> {}
