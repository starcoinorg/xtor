use std::{
    collections::HashMap,
    sync::{atomic::AtomicU64, Arc},
};

use anyhow::Result;
use futures::{
    channel::{
        mpsc::{self, UnboundedSender},
        oneshot,
    },
    FutureExt, StreamExt,
};
use lazy_static::lazy_static;
use tokio::{sync::RwLock, task::JoinHandle};

use super::{
    addr::{Addr, Event},
    context::Context,
};

pub(crate) static ACTOR_ID: AtomicU64 = AtomicU64::new(0);
pub(crate) static RUNNING_ACTOR_COUNTER: AtomicU64 = AtomicU64::new(0);

lazy_static! {
    pub static ref ACTOR_ID_NAME: RwLock<HashMap<u64, Option<String>>> =
        RwLock::new(HashMap::new());
    pub static ref ACTOR_ID_HANDLE: RwLock<HashMap<u64, JoinHandle<Result<()>>>> =
        RwLock::new(HashMap::new());
}

pub type ActorID = u64;

#[async_trait::async_trait]
pub trait Actor: Send + Sync + 'static {
    /// hook for actor initialization
    async fn on_start(&self, _ctx: &Context) -> Result<()> {
        Ok(())
    }
    /// hook for actor shutdown
    async fn on_stop(&self, _ctx: &Context) {}
    /// check the name of the actor
    async fn get_name(&self, ctx: &Context) -> Option<String> {
        ACTOR_ID_NAME.read().await[&ctx.id].clone()
    }
    /// starting an actor
    /// `?Sized` actor is not supported
    async fn spawn(self) -> Result<Addr>
    where
        Self: Sized,
    {
        ActorRunner::new().run(self).await
    }
}

pub struct ActorRunner {
    pub ctx: Context,
    tx: Arc<UnboundedSender<Event>>,
    rx: mpsc::UnboundedReceiver<Event>,
    tx_exit: oneshot::Sender<()>,
}

impl ActorRunner {
    pub fn new() -> Self {
        let (tx_exit, rx_exit) = oneshot::channel::<()>();
        let rx_exit = rx_exit.shared();
        let (ctx, rx, tx) = Context::new(rx_exit);
        Self {
            ctx,
            tx,
            rx,
            tx_exit,
        }
    }
    pub async fn run<A: Actor>(self, actor: A) -> Result<Addr> {
        let Self {
            ctx,
            mut rx,
            tx,
            tx_exit,
        } = self;

        let rx_exit = ctx.rx_exit.clone();
        let id = ctx.id;
        ACTOR_ID_NAME.write().await.insert(id, None);
        let actor = Arc::new(actor);
        actor.on_start(&ctx).await?;
        let handle = tokio::task::spawn(async move {
            let mut exit_err = Ok(());
            while let Some(event) = rx.next().await {
                match event {
                    Event::Stop(err) => {
                        exit_err = err;
                        break;
                    }
                    Event::Exec(f) => {
                        match f(actor.clone(), &ctx).await {
                            Ok(_) => {}
                            Err(err) => {
                                exit_err = Err(err);
                                break;
                            }
                        }
                    }
                }
            }
            actor.on_stop(&ctx).await;
            tx_exit.send(()).unwrap();
            exit_err
        });
        ACTOR_ID_HANDLE.write().await.insert(id, handle);
        let addr = Addr { id, tx, rx_exit };
        Ok(addr)
    }
}
