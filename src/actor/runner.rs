use std::{
    lazy::SyncLazy,
    sync::{atomic::AtomicU64, Arc},
};

use anyhow::Result;
use dashmap::DashMap;
use futures::{
    channel::{
        mpsc::{self, UnboundedSender},
        oneshot,
    },
    FutureExt, StreamExt,
};
use tokio::task::JoinHandle;
use tracing::{info, warn};

use super::{
    addr::{Addr, Event, WeakAddr},
    context::Context,
};

pub(crate) static ACTOR_ID: AtomicU64 = AtomicU64::new(0);

pub static ACTOR_ID_NAME: SyncLazy<DashMap<u64, Option<String>>> = SyncLazy::new(DashMap::new);
pub static ACTOR_ID_HANDLE: SyncLazy<DashMap<u64, JoinHandle<Result<()>>>> =
    SyncLazy::new(DashMap::new);

pub type ActorID = u64;

#[async_trait::async_trait]
pub trait Actor: Send + Sync + 'static {
    /// hook for actor initialization
    #[tracing::instrument(
        skip(self,_ctx),
        fields(addr = self.get_name_or_id_string(_ctx).as_str())
    )]
    async fn on_start(&self, _ctx: &Context) -> Result<()> {
        info!("{} start", self.get_name_or_id_string(_ctx));
        Ok(())
    }
    /// hook for actor shutdown
    #[tracing::instrument(
        skip(self,_ctx),
        fields(addr = self.get_name_or_id_string(_ctx).as_str())
    )]
    async fn on_stop(&self, _ctx: &Context) {
        info!("{} stop", self.get_name_or_id_string(_ctx));
    }
    /// check the name of the actor
    fn get_name(&self, ctx: &Context) -> Option<String> {
        ACTOR_ID_NAME.get(&ctx.id)?.clone()
    }
    fn get_name_or_id_string(&self, ctx: &Context) -> String {
        let name = self.get_name(ctx);
        if let Some(name) = name {
            format!("<{}:{}>", name, ctx.id)
        } else {
            format!("<anonymous actor:{}>", ctx.id)
        }
    }
    /// starting an actor
    /// if you want a supervised actor you need to send message to supervisor
    /// instead starting it from here `?Sized` actor is not supported
    async fn spawn(self) -> Result<Addr>
    where
        Self: Sized,
    {
        ActorRunner::new().run(self).await
    }

    async fn spawn_supervisable(self) -> Result<Addr>
    where
        Self: Sized + ActorRestart,
    {
        let addr = ActorRunner::new().supervised_run(self).await?;
        Ok(addr)
    }
}

pub struct ActorRunner {
    pub ctx: Context,
    tx: Arc<UnboundedSender<Event>>,
    rx: mpsc::UnboundedReceiver<Event>,
    tx_exit: oneshot::Sender<()>,
}

impl Default for ActorRunner {
    fn default() -> Self {
        Self::new()
    }
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
        ACTOR_ID_NAME.insert(id, None);
        let actor = Arc::new(actor);
        let addr = Addr { id, tx, rx_exit };
        ctx.addr.set(addr.downgrade()).expect("addr is already set");
        actor.on_start(&ctx).await?;
        let handle = tokio::task::spawn(async move {
            let mut exit_err = Ok(());
            while let Some(event) = rx.next().await {
                match event {
                    Event::Stop(err) => {
                        exit_err = err;
                        break;
                    }
                    Event::Exec(f) => match f(actor.clone(), &ctx).await {
                        Ok(_) => {}
                        Err(err) => {
                            exit_err = Err(err);
                            break;
                        }
                    },
                    Event::Restart => {
                        panic!("this event could only send by supervisor");
                    }
                    Event::AddSupervisor(_) => {
                        panic!("this event could only send by supervisor");
                    }
                }
            }
            actor.on_stop(&ctx).await;
            tx_exit.send(()).expect("tx_exit is already closed");
            ACTOR_ID_HANDLE.remove(&id);
            exit_err
        });
        ACTOR_ID_HANDLE.insert(id, handle);
        Ok(addr)
    }

    pub(crate) async fn supervised_run<A: Actor + ActorRestart>(self, actor: A) -> Result<Addr> {
        let Self {
            ctx,
            mut rx,
            tx,
            tx_exit,
        } = self;

        let rx_exit = ctx.rx_exit.clone();
        let id = ctx.id;
        ACTOR_ID_NAME.insert(id, None);
        let actor = Arc::new(actor);
        let addr = Addr { id, tx, rx_exit };
        ctx.addr.set(addr.downgrade()).expect("addr is already set");
        actor.on_start(&ctx).await?;
        let weakaddr = addr.downgrade();
        let handle = tokio::task::spawn(async move {
            let mut exit_err = Ok(());
            'supervising_loop: loop {
                'event_loop: while let Some(event) = rx.next().await {
                    match event {
                        Event::Stop(err) => {
                            exit_err = err;
                            break 'event_loop;
                        }
                        Event::Exec(f) => match f(actor.clone(), &ctx).await {
                            Ok(_) => {}
                            Err(err) => {
                                exit_err = Err(err);
                                break 'event_loop;
                            }
                        },
                        Event::Restart => {
                            actor.on_restart(&weakaddr).await;
                            continue 'supervising_loop;
                        }
                        Event::AddSupervisor(proxy) => {
                            ctx.supervisors.lock().await.push(proxy);
                        }
                    }
                }
                // supervice logic
                if exit_err.is_err() {
                    exit_err = ctx.await_supervisor().await;
                    if exit_err.is_err() {
                        break 'supervising_loop;
                    } else {
                        actor.on_restart(&weakaddr).await;
                        continue 'supervising_loop;
                    }
                } else {
                    break 'supervising_loop;
                }
            }
            actor.on_stop(&ctx).await;
            tx_exit.send(()).expect("tx_exit is already closed");
            ACTOR_ID_HANDLE.remove(&id);
            exit_err
        });
        ACTOR_ID_HANDLE.insert(id, handle);
        Ok(addr)
    }
}

/// the default behavior of restarting an actor
/// WARNING: default is do nothing
#[async_trait::async_trait]
pub trait ActorRestart {
    #[tracing::instrument(
        skip(self),
        fields(addr = _addr.get_name_or_id_string().as_str())
    )]
    async fn on_restart(&self, _addr: &WeakAddr) {
        warn!("{} restarted", _addr.get_name_or_id_string().as_str());
    }
}
