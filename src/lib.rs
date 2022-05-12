#![feature(type_name_of_val)]

use actor::runner::ACTOR_ID_HANDLE;

use futures::Future;

#[cfg(test)]
mod tests;

pub use xtor_derive::main;
pub use xtor_derive::message;

pub mod actor;
pub mod utils;

pub use actor::*;

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

pub fn block_on<F, T>(future: F) -> T
where
    F: Future<Output = T>,
{
    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    rt.block_on(future)
}
