use std::hash::Hash;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Weak;
use std::time::Duration;

use super::actor::ActorID;
use super::actor::ACTOR_ID_NAME;
use super::context::Context;
use super::message::Handler;
use super::message::Message;
use super::proxy::Proxy;
use super::proxy::ProxyFnBlock;
use anyhow::Result;
use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::future::Shared;
use futures::lock::Mutex;
use futures::Future;

pub type ExecFuture<'a> = Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;
pub type ExecFn =
    Box<dyn FnOnce(Arc<dyn std::any::Any + Send + Sync>, &Context) -> ExecFuture + Send + 'static>;

/// default wait interval to 10ms
/// you can set to a custom value
pub static mut ACTOR_STOP_WAIT_INTERVAL: Duration = std::time::Duration::from_millis(10);

pub enum Event {
    Stop(Result<()>),
    Exec(ExecFn),
    // TODO: Link to supervisor
    // TODO: Subscribe to Broker
}

#[derive(Clone)]
pub struct Addr {
    pub id: ActorID,
    pub(crate) tx: Arc<mpsc::UnboundedSender<Event>>,
    pub(crate) rx_exit: Shared<oneshot::Receiver<()>>,
}

impl Addr {
    /// send stop event to the actor
    pub fn stop(self, err: Result<()>) {
        let _ = mpsc::UnboundedSender::clone(&*self.tx).start_send(Event::Stop(err));
    }
    /// Raw exec is not recommended to use, please use `call` or `send` instead
    pub fn exec(self, f: ExecFn) {
        mpsc::UnboundedSender::clone(&*self.tx)
            .start_send(Event::Exec(f))
            .unwrap();
    }

    pub async fn call<A: Handler<T>, T: Message>(&self, msg: T) -> anyhow::Result<T::Result> {
        self.call_unblock::<A, T>(msg)
            .await?
            .await
            .map_err(|e| e.into())
    }

    pub async fn call_unblock<A: Handler<T>, T: Message>(
        &self,
        msg: T,
    ) -> anyhow::Result<oneshot::Receiver<T::Result>> {
        let (tx, rx) = oneshot::channel();
        mpsc::UnboundedSender::clone(&*self.tx).start_send(Event::Exec(Box::new(move |actor, ctx| {
            Box::pin(async move {
                let handler = match actor.as_ref().downcast_ref::<A>() {
                    Some(handler) => Ok(handler),
                    None => Err(anyhow::anyhow!("error: {} trying to handle a message in actor which you didn't implement the handler trait {} for it", std::any::type_name_of_val(&actor), std::any::type_name::<dyn Handler::<T>>())),
                }?;
                let res = handler.handle(ctx, msg).await?;
                let _ = tx.send(res);
                Ok(())
            })
        })))?;
        Ok(rx)
    }

    pub async fn proxy<A: Handler<T>, T: Message>(&self) -> Proxy<T> {
        let weak_tx = Arc::downgrade(&self.tx);
        let inner: ProxyFnBlock<T> = Box::new(move |msg| {
            let weak_tx = weak_tx.clone();
            Box::pin(async move {
                let (tx, rx) = oneshot::channel();
                let ttx = weak_tx
                    .upgrade()
                    .ok_or_else(|| anyhow::anyhow!("error: proxy tx is dropped"))?;
                mpsc::UnboundedSender::clone(&*ttx).start_send(Event::Exec(Box::new(move |actor, ctx| {
                        Box::pin(async move {
                            let handler = match actor.as_ref().downcast_ref::<A>() {
                                Some(handler) => Ok(handler),
                                None => Err(anyhow::anyhow!("error: {} trying to handle a message in actor which you didn't implement the handler trait {} for it", std::any::type_name_of_val(&actor), std::any::type_name::<dyn Handler::<T>>())),
                            }?;
                            let res = handler.handle(ctx, msg).await?;
                            let _ = tx.send(res);
                            Ok(())
                        })
                    })))?;
                Ok(rx.await?)
            })
        });

        Proxy {
            id: self.id.clone(),
            proxy_inner: Mutex::new(inner),
        }
    }

    /// check if it is stoped
    /// the default interval is 10ms
    pub async fn is_stopped(&self) -> bool {
        tokio::select! {
            _ = self.rx_exit.clone() => true,
            _ = tokio::time::sleep(unsafe{ACTOR_STOP_WAIT_INTERVAL}) => false,
        }
    }

    /// wait to stop
    /// used in main method for blocking the main thread
    pub async fn await_stop(&self) -> Result<()> {
        self.rx_exit.clone().await.map_err(|err| err.into())
    }

    /// set the name of the actor
    /// better for debug
    pub async fn set_name<T: Into<String>>(&self, name: T) {
        ACTOR_ID_NAME
            .write()
            .await
            .insert(self.id, Some(name.into()));
    }

    pub fn downgrade(&self) -> WeakAddr {
        WeakAddr {
            id: self.id.clone(),
            _tx: Arc::downgrade(&self.tx),
            _rx_exit: self.rx_exit.clone(),
        }
    }
}

impl std::fmt::Debug for Addr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<Addr: {}>", self.id)
    }
}

impl Hash for Addr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

pub struct WeakAddr {
    pub id: ActorID,
    pub(crate) _tx: Weak<mpsc::UnboundedSender<Event>>,
    pub(crate) _rx_exit: Shared<oneshot::Receiver<()>>,
}

impl WeakAddr {
    pub fn upgrade(&self) -> Option<Addr> {
        if let Some(tx) = self._tx.upgrade() {
            Some(Addr {
                id: self.id.clone(),
                tx: tx,
                rx_exit: self._rx_exit.clone(),
            })
        } else {
            None
        }
    }
}

impl std::fmt::Debug for WeakAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<Addr?: {}>", self.id)
    }
}

impl Hash for WeakAddr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
