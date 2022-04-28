use std::sync::{Arc, Weak};

use futures::{
    channel::{mpsc, oneshot},
    future::Shared,
};

use super::{
    actor::{ActorID, ACTOR_ID, RUNNING_ACTOR_COUNTER},
    addr::Event,
};

pub struct Context {
    pub id: ActorID,
    tx: Weak<mpsc::UnboundedSender<Event>>,
    pub(crate) rx_exit: Shared<oneshot::Receiver<()>>,
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
}

impl Drop for Context {
    fn drop(&mut self) {
        RUNNING_ACTOR_COUNTER.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}
