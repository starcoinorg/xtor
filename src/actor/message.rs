use std::sync::Arc;

use super::{actor::Actor, context::Context};
use anyhow::Result;
use futures::channel::mpsc;

pub trait Message: 'static + Send {
    type Result: 'static + Send;
}

#[async_trait::async_trait]
pub trait Handler<T: Message>: Actor {
    async fn handle(&self, ctx: &Context, msg: T) -> Result<T::Result>;
}

#[async_trait::async_trait]
impl<T: Message> Handler<T> for Arc<dyn Handler<T>> {
    async fn handle(&self, ctx: &Context, msg: T) -> Result<T::Result> {
        (**self).handle(ctx, msg).await
    }
}

/// warning! please do not use this as a Actor due to this is just a wrapper for handling message
#[async_trait::async_trait]
impl<T: Message> Actor for Arc<dyn Handler<T>> {}

#[async_trait::async_trait]
pub trait HandlerStreamEXT<T: Message>: Actor {
    async fn on_poll(&self, ctx: &Context, msg: T) -> Result<()>;
    async fn on_finished(&self, ctx: &Context) -> Result<()>;
    async fn get_handled_stream(&self) -> Result<mpsc::UnboundedReceiver<T>>;
}
