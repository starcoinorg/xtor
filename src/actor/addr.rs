use std::hash::Hash;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Weak;
use std::time::Duration;

use crate::Supervise;

use super::context::Context;
use super::message::Handler;
use super::message::Message;
use super::proxy::Proxy;
use super::proxy::ProxyFnBlock;
use super::runner::ActorID;
use super::runner::ACTOR_ID_NAME;
use super::supervisor::Restart;
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
    Restart,
    AddSupervisor(Proxy<Restart>),
}

#[derive(Clone)]
pub struct Addr {
    pub id: ActorID,
    pub(crate) tx: Arc<mpsc::UnboundedSender<Event>>,
    pub(crate) rx_exit: Shared<oneshot::Receiver<()>>,
}

impl Addr {
    pub async fn link_to_supervisor(&self, proxy: &Proxy<Supervise>) -> Result<()> {
        proxy.call(Supervise(self.clone())).await?;
        Ok(())
    }
    pub async fn chain_link_to_supervisor(self, proxy: &Proxy<Supervise>) -> Result<Self> {
        proxy.call(Supervise(self.clone())).await?;
        Ok(self)
    }

    pub async fn get_name_or_id_string(&self) -> String {
        let name = self.get_name().await;
        if let Some(name) = name {
            name
        } else {
            self.id.to_string()
        }
    }

    pub async fn get_name(&self) -> Option<String> {
        ACTOR_ID_NAME.read().await[&self.id].clone()
    }

    pub async fn add_supervisor(&self, supervisor: Proxy<Restart>) {
        let _ =
            mpsc::UnboundedSender::clone(&*self.tx).start_send(Event::AddSupervisor(supervisor));
    }

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
        self.call_unblock::<A, T>(msg).await.await?
    }

    pub async fn call_unblock<A: Handler<T>, T: Message>(
        &self,
        msg: T,
    ) -> oneshot::Receiver<anyhow::Result<T::Result>> {
        let (tx, rx) = oneshot::channel();
        let _ = mpsc::UnboundedSender::clone(&*self.tx).start_send(Event::Exec(Box::new(move |actor, ctx| {
            Box::pin(async move {
                match actor.as_ref().downcast_ref::<A>() {
                    Some(handler) => {
                        match handler.handle(ctx, msg).await {
                            Ok(res) => {let _ = tx.send(Ok(res));Ok(())},
                            Err(e) => Err(e),
                        }
                    },
                    None => Err(anyhow::anyhow!("error: {} trying to handle a message in actor which you didn't implement the handler trait {} for it", std::any::type_name_of_val(&actor), std::any::type_name::<dyn Handler::<T>>())),
                }
            })
        })));
        rx
    }

    /// Ok(None) means it is timeout
    /// Ok(Some(res)) means it is not timeout
    /// Err(e) means it is not timeout but with error occurred
    pub async fn call_timeout<A: Handler<T>, T: Message>(
        &self,
        msg: T,
        timeout: Duration,
    ) -> anyhow::Result<Option<T::Result>> {
        let chan = self.call_unblock::<A, T>(msg).await;
        tokio::select! {
            res = chan =>  {
                res.map(|x| x.ok()).map_err(|e| e.into())
            }
            _ = tokio::time::sleep(timeout) => Ok(None)
        }
    }

    /// create a proxy (like delegate in C#)
    /// you only needs to care the lifetime and the type when you trying to create
    /// when you calling to proxy, the type of actor is not needed.
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
                        match actor.as_ref().downcast_ref::<A>() {
                            Some(handler) => {
                                match handler.handle(ctx, msg).await {
                                    Ok(res) => {let _ = tx.send(Ok(res));Ok(())},
                                    Err(e) => Err(e),
                                }
                            },
                            None => Err(anyhow::anyhow!("error: {} trying to handle a message in actor which you didn't implement the handler trait {} for it", std::any::type_name_of_val(&actor), std::any::type_name::<dyn Handler::<T>>())),
                        }
                    })
                    })))?;
                Ok(rx)
            })
        });

        Proxy {
            id: self.id,
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
            id: self.id,
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
        self._tx.upgrade().map(|tx| Addr {
            id: self.id,
            tx,
            rx_exit: self._rx_exit.clone(),
        })
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
