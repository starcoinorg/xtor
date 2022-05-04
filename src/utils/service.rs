use std::{any::TypeId, sync::Arc};

use anyhow::Result;
use dashmap::DashMap;
use lazy_static::lazy_static;

use crate::actor::{actor::Actor, addr::Addr};

pub type Registry = DashMap<TypeId, Arc<dyn Service>>;

lazy_static! {
    static ref GLOBAL_SERVICE_REGISTRY: Registry = DashMap::new();
}

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
