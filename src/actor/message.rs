use std::sync::Arc;

use super::{actor::Actor, context::Context};
use anyhow::Result;


pub trait Message: 'static + Send {
    type Result: 'static + Send;
}

#[async_trait::async_trait]
pub trait Handler<T: Message>: Actor {
    async fn handle(&self, ctx: &Context, msg: T) -> Result<T::Result>;
}

/// warning! please do not use this as an Actor due to this is just a wrapper for handling message
#[async_trait::async_trait]
impl<T: Message> Handler<T> for Arc<dyn Handler<T>> {
    async fn handle(&self, ctx: &Context, msg: T) -> Result<T::Result> {
        (**self).handle(ctx, msg).await
    }
}

/// warning! please do not use this as an Actor due to this is just a wrapper for handling message
#[async_trait::async_trait]
impl<T: Message> Actor for Arc<dyn Handler<T>> {}
