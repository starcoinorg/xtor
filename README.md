# Xtor: An handler async actor framework.

[![CI](https://github.com/starcoinorg/xtor/actions/workflows/rust.yml/badge.svg)](https://github.com/starcoinorg/xtor/actions/workflows/rust.yml)
[![Crates](https://img.shields.io/crates/v/xtor)](https://crates.io/crates/xtor)

## Key features

- small: very small codebase
- async: allow you to write async code in your actor
- full featured: we have built-in types such as `Supervisor` `Broker` `Caller` and so on
- both dynamic and fast: typed message and weak typed event.

## usage

add `xtor` to your library

```
cargo add xtor
```

write some code

```rs
use anyhow::Result;
use async_trait::async_trait;
use xtor::actor::{context::Context, message::Handler, runner::Actor};

// first define actor
struct HelloActor;
impl Actor for HelloActor {}

// then define message
#[xtor::message(result = "()")]
#[derive(Debug)]
struct Hello;

// then impl the handler
#[async_trait]
impl Handler<Hello> for HelloActor {
    async fn handle(&self, _ctx: &Context, msg: Hello) -> Result<()> {
        println!("{:?} received", &msg);
        Ok(())
    }
}

// main will finish when all actors died out.
#[xtor::main]
async fn main() -> Result<()> {
    let hello_actor = HelloActor;
    let hello_actor_address = hello_actor.spawn().await?;
    hello_actor_address.call::<HelloActor, Hello>(Hello).await
}
```

## project structure

- `src/actor/*` for pure async actor implementation
- `src/utils/*` for utilities both trait and default implementation such as
  - `DefaultBroker`
  - `DefaultSupervisor`
  - `Service`

## References

- [Actix](https://github.com/actix/actix)
- [Xactor](https://github.dev/sunli829/xactor)
- [Tokio](https://github.com/tokio-rs/tokio)
