// broker is also a actor
// broker is also a service

// broker has message publish implemented
// broker could return a proxy
// broker call will return a spmc channel
// actor an subscribe to brokers after it implement the handler

use crate::actor::{
    addr::{Addr, WeakAddr},
    message::{Handler, Message},
    proxy::Proxy,
};

/// the trait of broker
#[async_trait::async_trait]
pub trait Broker<T: Message + Sync + Clone>:
    Handler<Publish<T>> + Handler<Subscribe<T>> + Handler<Unsubscribe>
{
}

/// for publishing message to subscribers
pub struct Publish<T: Message + Sync + Clone + Clone>(pub T);

impl<T: Message + Sync + Clone> Message for Publish<T> {
    type Result = ();
}

pub type SubscriptionID = u64;

/// subscribe to a broker
pub struct Subscribe<T: Message + Sync + Clone> {
    pub addr: WeakAddr,
    pub proxy: Proxy<T>,
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

/// unsubscribe from a broker
pub struct Unsubscribe(pub SubscriptionID);
impl Message for Unsubscribe {
    type Result = ();
}
