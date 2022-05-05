use std::sync::{Arc, Weak};

use futures::{
    channel::{mpsc, oneshot},
    future::{join_all, Shared},
};
use once_cell::sync::OnceCell;
use tokio::sync::RwLock;

use super::{
    runner::{ActorID, ACTOR_ID, RUNNING_ACTOR_COUNTER},
    addr::{Event, WeakAddr},
    proxy::Proxy,
    supervisor::Restart,
};

pub struct Context {
    pub id: ActorID,
    tx: Weak<mpsc::UnboundedSender<Event>>,
    pub(crate) rx_exit: Shared<oneshot::Receiver<()>>,
    pub(crate) supervisors: RwLock<Vec<Proxy<Restart>>>,
    pub(crate) addr: OnceCell<WeakAddr>,
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

impl Context {
    pub fn new(
        rx_exit: Shared<oneshot::Receiver<()>>,
    ) -> (
        Self,
        mpsc::UnboundedReceiver<Event>,
        Arc<mpsc::UnboundedSender<Event>>,
    ) {
        let id = ACTOR_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (tx, rx) = mpsc::unbounded();
        let tx = Arc::new(tx);
        let weak_tx = Arc::downgrade(&tx);
        RUNNING_ACTOR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        (
            Self {
                id,
                tx: weak_tx,
                rx_exit,
                supervisors: RwLock::new(vec![]),
                addr: OnceCell::new(),
            },
            rx,
            tx,
        )
    }

    pub fn stop(&self) {
        if let Some(tx) = self.tx.upgrade() {
            tx.unbounded_send(Event::Stop(Ok(()))).unwrap();
        }
    }

    pub async fn await_supervisor(&self) -> anyhow::Result<()> {
        // check if at least one supervisor is alive
        // first result error: dead
        // second result error: alive but failed to execute restart
        // any second result error: fail
        let supervisors = self.supervisors.read().await;
        if supervisors.len() > 0 {
            let jobs = supervisors.iter().map(|p| p.call(Restart(self.id)));
            let mut restarts = join_all(jobs)
                .await
                .into_iter()
                .filter(|r| r.is_ok())
                .map(|r| r.unwrap());
            if restarts.any(|r| r.is_err()) {
                Err(anyhow::anyhow!(
                    "one or more supervisor failed to restart {:?}",
                    restarts
                ))
            } else {
                Ok(())
            }
        } else {
            Err(anyhow::anyhow!("no supervisor was found"))
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        RUNNING_ACTOR_COUNTER.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}
