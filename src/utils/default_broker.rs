use std::{marker::PhantomData, sync::atomic::AtomicU64};

use dashmap::DashMap;
use futures::{Stream, StreamExt};
use tokio::task::JoinHandle;

use crate::{
    broker::{Broker, Publish, Subscribe, SubscriptionID, Unsubscribe},
    Actor, Addr, Context, Handler, Message,
};

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

pub struct StreamBroker<S: Stream<Item = I> + Sync + Send + 'static, I: Message + Clone>(pub S);
impl<S: Stream<Item = I> + Sync + Send + 'static, I: Message + Clone> Actor for StreamBroker<S, I> {}
impl<S: Stream<Item = I> + Sync + Send + 'static + Unpin, I: Message + Sync + Clone>
    StreamBroker<S, I>
{
    pub async fn spawn(mut self) -> anyhow::Result<(Addr, JoinHandle<anyhow::Result<()>>)> {
        let broker = DefaultBroker::<I>::new().spawn().await?;
        let broker_a = broker.clone();
        Ok((
            broker,
            tokio::spawn(async move {
                while let Some(msg) = self.0.next().await {
                    broker_a
                        .call::<DefaultBroker<I>, Publish<I>>(Publish(msg))
                        .await?;
                }
                Ok(())
            }),
        ))
    }
}
