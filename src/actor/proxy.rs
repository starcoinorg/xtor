use std::pin::Pin;

use futures::{lock::Mutex, Future};

use anyhow::Result;

use super::{actor::ActorID, message::Message};

pub(crate) type ProxyRetBlock<T> =
    Pin<Box<dyn Future<Output = Result<<T as Message>::Result>> + Send + 'static>>;
pub(crate) type ProxyFnBlock<T> = Box<dyn Fn(T) -> ProxyRetBlock<T> + Send + Sync + 'static>;

pub struct Proxy<T: Message> {
    pub id: ActorID,
    pub proxy_inner: Mutex<ProxyFnBlock<T>>,
}

impl<T: Message> Proxy<T> {
    pub fn new(id: ActorID, proxy_inner: ProxyFnBlock<T>) -> Self {
        Self {
            id,
            proxy_inner: Mutex::new(proxy_inner),
        }
    }

    pub async fn call(&self, msg: T) -> Result<T::Result> {
        self.proxy_inner.lock().await(msg).await
    }

    pub async fn call_timeout(
        &self,
        msg: T,
        timeout: std::time::Duration,
    ) -> Result<Option<T::Result>> {
        tokio::select! {
            res = self.proxy_inner.lock().await(msg) => {
            res.map_err(|e| e.into()).map(|x| Some(x))
        }
            _ = tokio::time::sleep(timeout) => Ok(None)
        }
    }

    pub async fn call_unblock(&self, msg: T) -> ProxyRetBlock<T> {
        self.proxy_inner.lock().await(msg)
    }
}
