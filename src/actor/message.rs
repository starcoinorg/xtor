use std::sync::Arc;

use anyhow::Result;

use super::{context::Context, runner::Actor};

/// message trait
/// like args in function call
pub trait Message: 'static + Send {
    type Result: 'static + Send;
}

/// handle message for actor
#[async_trait::async_trait]
pub trait Handler<T: Message>: Actor {
    async fn handle(&self, ctx: &Context, msg: T) -> Result<T::Result>;
}

/// warning! please do not use this as an Actor due to this is just a wrapper
/// for handling message
#[async_trait::async_trait]
impl<T: Message> Handler<T> for Arc<dyn Handler<T>> {
    async fn handle(&self, ctx: &Context, msg: T) -> Result<T::Result> {
        (**self).handle(ctx, msg).await
    }
}

/// warning! please do not use this as an Actor due to this is just a wrapper
/// for handling message
#[async_trait::async_trait]
impl<T: Message> Actor for Arc<dyn Handler<T>> {}
