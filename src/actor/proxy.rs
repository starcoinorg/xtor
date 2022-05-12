use std::pin::Pin;

use futures::{channel::oneshot, Future};

use anyhow::Result;

use super::{message::Message, runner::ActorID};

pub(crate) type ProxyRetBlock<T> = Pin<
    Box<
        dyn Future<Output = Result<oneshot::Receiver<Result<<T as Message>::Result>>>>
            + Send
            + 'static,
    >,
>;
pub(crate) type ProxyFnBlock<T> = Box<dyn Fn(T) -> ProxyRetBlock<T> + Send + Sync + 'static>;

pub struct Proxy<T: Message> {
    pub id: ActorID,
    pub proxy_inner: ProxyFnBlock<T>,
}

impl<T: Message> Proxy<T> {
    pub fn new(id: ActorID, proxy_inner: ProxyFnBlock<T>) -> Self {
        Self { id, proxy_inner }
    }

    pub async fn call(&self, msg: T) -> Result<T::Result> {
        (self.proxy_inner)(msg).await?.await?
    }

    pub async fn call_timeout(
        &self,
        msg: T,
        timeout: std::time::Duration,
    ) -> Result<Option<T::Result>> {
        tokio::select! {
            res =( self.proxy_inner)(msg).await? => {
                res?.map( Some )
            }
            _ = tokio::time::sleep(timeout) => Ok(None)
        }
    }

    pub async fn call_unblock(&self, msg: T) -> ProxyRetBlock<T> {
        (self.proxy_inner)(msg)
    }
}
