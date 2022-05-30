use std::{any::TypeId, lazy::SyncLazy, sync::Arc};

use anyhow::Result;
use dashmap::DashMap;

use crate::actor::{addr::Addr, runner::Actor};

pub type Registry = DashMap<TypeId, Arc<dyn Service>>;

pub static GLOBAL_SERVICE_REGISTRY: SyncLazy<Registry> = SyncLazy::new(DashMap::new);

thread_local! {
    static LOCAL_SERVICE_REGISTRY: Registry = DashMap::new();
}

#[async_trait::async_trait]
pub trait Service: Actor {
    async fn from_registry(registry: &Registry) -> Result<Addr>
    where
        Self: Sized;
    async fn from_local_registry() -> Result<Addr>
    where
        Self: Sized;
    async fn from_global_registry() -> Result<Addr>
    where
        Self: Sized;
}
