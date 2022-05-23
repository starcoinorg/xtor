use std::time::Duration;

use dashmap::DashMap;
use futures::{channel::mpsc, lock::Mutex};
use tracing::info;

use crate::actor::{
    addr::Addr,
    context::Context,
    message::Handler,
    runner::{Actor, ActorID},
    supervisor::{Restart, Supervise, Supervisor, Unsupervise},
};

pub struct DefaultSupervisor {
    restart_strategy: DefaultSupervisorRestartStrategy,
    last_restart: Mutex<std::time::Instant>,
    restart_delay: Duration,
    supervised_actors: DashMap<ActorID, Addr>,
}

pub enum DefaultSupervisorRestartStrategy {
    OneForOne,
    OneForAll,
}

impl DefaultSupervisor {
    pub fn new(restart_strategy: DefaultSupervisorRestartStrategy) -> Self {
        Self {
            restart_strategy,
            last_restart: Mutex::new(std::time::Instant::now()),
            restart_delay: Duration::from_millis(100),
            supervised_actors: DashMap::new(),
        }
    }
}

#[async_trait::async_trait]
impl Actor for DefaultSupervisor {
    #[tracing::instrument(
        skip(self, _ctx),
        fields(addr = self.get_name_or_id_string(_ctx).as_str())
    )]
    async fn on_stop(&self, _ctx: &Context) {
        info!("{} stop", self.get_name_or_id_string(_ctx));
        self.supervised_actors.clear();
    }
}

#[async_trait::async_trait]
impl Handler<Restart> for DefaultSupervisor {
    async fn handle(&self, _ctx: &Context, msg: Restart) -> anyhow::Result<anyhow::Result<()>> {
        let mut last_restart = self.last_restart.lock().await;
        while last_restart.elapsed() < self.restart_delay {
            tokio::time::sleep(self.restart_delay - last_restart.elapsed()).await;
        }
        let mut inner_err = Ok(());
        match self.restart_strategy {
            DefaultSupervisorRestartStrategy::OneForOne => {}
            DefaultSupervisorRestartStrategy::OneForAll => {
                for addr in &self.supervised_actors {
                    if addr.id != msg.0 {
                        inner_err = mpsc::UnboundedSender::clone(&*addr.tx)
                            .start_send(crate::actor::addr::Event::Restart);
                    }
                }
            }
        }

        *last_restart = std::time::Instant::now();
        Ok(inner_err.map_err(|e| e.into()))
    }
}

#[async_trait::async_trait]
impl Handler<Supervise> for DefaultSupervisor {
    async fn handle(&self, ctx: &Context, msg: Supervise) -> anyhow::Result<()> {
        msg.0
            .add_supervisor(
                ctx.addr
                    .get()
                    .expect("supervisor should be in the context")
                    .upgrade()
                    .expect("supervisor should be in the context")
                    .proxy::<DefaultSupervisor, Restart>()
                    .await,
            )
            .await;
        self.supervised_actors.insert(msg.0.id, msg.0);
        Ok(())
    }
}

#[async_trait::async_trait]
impl Handler<Unsupervise> for DefaultSupervisor {
    async fn handle(&self, _ctx: &Context, msg: Unsupervise) -> anyhow::Result<()> {
        self.supervised_actors.remove(&msg.0.id);
        Ok(())
    }
}

impl Supervisor for DefaultSupervisor {}
