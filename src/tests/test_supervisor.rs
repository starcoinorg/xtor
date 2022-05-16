use std::sync::atomic::{AtomicBool, AtomicUsize};

use crate::{utils::default_supervisor::DefaultSupervisor, ActorRestart, Supervise, WeakAddr};

use super::*;

struct Dummy(AtomicBool, AtomicUsize);
impl Default for Dummy {
    fn default() -> Self {
        Dummy(AtomicBool::new(false), AtomicUsize::new(0))
    }
}
impl Actor for Dummy {}

#[crate::message(result = "()")]
struct Die;

#[async_trait::async_trait]
impl Handler<Die> for Dummy {
    async fn handle(&self, _ctx: &Context, _msg: Die) -> anyhow::Result<()> {
        println!("Dummy actor is died");
        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
        Err(anyhow::anyhow!("dummy died"))
    }
}

#[crate::message(result = "(bool, usize)")]
struct IsAlive;

#[async_trait::async_trait]
impl Handler<IsAlive> for Dummy {
    async fn handle(&self, _ctx: &Context, _msg: IsAlive) -> anyhow::Result<(bool, usize)> {
        Ok((
            self.0.load(std::sync::atomic::Ordering::SeqCst),
            self.1.load(std::sync::atomic::Ordering::SeqCst),
        ))
    }
}

#[async_trait::async_trait]
impl ActorRestart for Dummy {
    async fn on_restart(&self, _addr: &WeakAddr) {
        println!("restarting Dummy");
        self.0.store(false, std::sync::atomic::Ordering::SeqCst);
        self.1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[crate::test]
async fn test_supervisor_save_one() {
    let supervisor = DefaultSupervisor::new(
        xtor::utils::default_supervisor::DefaultSupervisorRestartStrategy::OneForOne,
    )
    .spawn()
    .await
    .unwrap();
    let supervise_proxy = supervisor.proxy::<DefaultSupervisor, Supervise>().await;
    let actor = Dummy::default()
        .spawn_supervisable()
        .await
        .unwrap()
        .chain_link_to_supervisor(&supervise_proxy)
        .await
        .unwrap();
    let _ = actor.call::<Dummy, Die>(Die).await;
    assert!(!actor.is_stopped().await);
    assert!(!actor.call::<Dummy, IsAlive>(IsAlive).await.unwrap().0);
}

#[crate::test]
async fn test_supervisor_save_all() {
    let supervisor = DefaultSupervisor::new(
        xtor::utils::default_supervisor::DefaultSupervisorRestartStrategy::OneForAll,
    )
    .spawn()
    .await
    .unwrap();
    let supervise_proxy = supervisor.proxy::<DefaultSupervisor, Supervise>().await;
    let actors = futures::future::join_all((0..10).map(|_| async {
        Dummy::default()
            .spawn_supervisable()
            .await
            .unwrap()
            .chain_link_to_supervisor(&supervise_proxy)
            .await
            .unwrap()
    }))
    .await;
    let _ = actors.iter().next().unwrap().call::<Dummy, Die>(Die).await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    for a in actors {
        assert_eq!(a.call::<Dummy, IsAlive>(IsAlive).await.unwrap().1, 1);
    }
}
