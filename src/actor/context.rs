use std::{
    lazy::SyncOnceCell,
    sync::{Arc, Weak},
};

use futures::{
    channel::{mpsc, oneshot},
    future::{join_all, Shared},
    lock::Mutex,
};

use super::{
    addr::{Event, WeakAddr},
    proxy::Proxy,
    runner::{ActorID, ACTOR_ID},
    supervisor::Restart,
};

/// the context of an actor
pub struct Context {
    pub id: ActorID,
    tx: Weak<mpsc::UnboundedSender<Event>>,
    pub(crate) rx_exit: Shared<oneshot::Receiver<()>>,
    pub(crate) supervisors: Mutex<Vec<Proxy<Restart>>>,
    pub(crate) addr: SyncOnceCell<WeakAddr>,
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
        (
            Self {
                id,
                tx: weak_tx,
                rx_exit,
                supervisors: Mutex::new(vec![]),
                addr: SyncOnceCell::new(),
            },
            rx,
            tx,
        )
    }

    /// to stop an actor
    pub fn stop(&self) {
        if let Some(tx) = self.tx.upgrade() {
            tx.unbounded_send(Event::Stop(Ok(())))
                .expect("failed to send stop event");
        }
    }

    /// await supervisor to restart an actor
    pub async fn await_supervisor(&self) -> anyhow::Result<()> {
        // check if at least one supervisor is alive
        // first result error: dead
        // second result error: alive but failed to execute restart
        // any second result error: fail
        let supervisors = self.supervisors.lock().await;
        if supervisors.len() > 0 {
            let jobs = supervisors.iter().map(|p| p.call(Restart(self.id)));
            let mut restarts = join_all(jobs)
                .await
                .into_iter()
                .filter(|r| r.is_ok())
                .map(|r| r.expect("failed to restart"));
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
