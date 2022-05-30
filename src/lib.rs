//! # Xtor: An handler async actor framework.
//! [![Rust](https://github.com/starcoinorg/xtor/actions/workflows/rust.yml/badge.svg)](https://github.com/starcoinorg/xtor/actions/workflows/rust.yml)
//!
//! ## Key features
//! - small: very small codebase
//! - async: allow you to write async code in your actor
//! - full featured: we have built-in types such as `Supervisor` `Broker` `Caller` and so on
//! - both dynamic and fast: typed message and weak typed event.
//!
//! ## usage
//!
//! add `xtor` to your library
//! ```sh
//! cargo add xtor
//! ```
//!
//! write some code
//! ```
//! use anyhow::Result;
//! use async_trait::async_trait;
//! use xtor::actor::{context::Context, message::Handler, runner::Actor};
//!
//! // first define actor
//! struct HelloActor;
//! impl Actor for HelloActor {}
//!
//! // then define message
//! #[xtor::message(result = "()")]
//! #[derive(Debug)]
//! struct Hello;
//!
//! // then impl the handler
//! #[async_trait]
//! impl Handler<Hello> for HelloActor {
//!     async fn handle(&self, _ctx: &Context, msg: Hello) -> Result<()> {
//!         println!("{:?} received", &msg);
//!         Ok(())
//!     }
//! }
//!
//! // main will finish when all actors died out.
//! #[xtor::main]
//! async fn main() -> Result<()> {
//!     let hello_actor = HelloActor;
//!     let hello_actor_address = hello_actor.spawn().await?;
//!     hello_actor_address.call::<HelloActor, Hello>(Hello).await
//! }
//! ```
//!
//! ## More Examples?
//! please take a look at the examples folder in the [repository](https://github.com/starcoinorg/xtor).
//!

#![feature(type_name_of_val)]

use actor::runner::ACTOR_ID_HANDLE;

use futures::Future;

#[cfg(test)]
mod tests;

pub use xtor_derive::main;
pub use xtor_derive::message;
pub use xtor_derive::test;

/// the core of xtor
pub mod actor;
/// default implemention of broker, supervisor and so on.
pub mod utils;

pub use actor::*;

/// exports to the derive macro
#[inline(always)]
pub async fn await_exit() {
    loop {
        if ACTOR_ID_HANDLE.is_empty() {
            break;
        } else {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
    }
}

/// exports to the derive macro
pub fn block_on<F, T>(future: F) -> T
where
    F: Future<Output = T>,
{
    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    rt.block_on(future)
}
