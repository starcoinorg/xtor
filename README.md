# Xtor: An actor async actor framework.

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
use async_trait::async_trait;
use xtor::actor::{actor::Actor, message::Handler, context::Context};
use anyhow::Result;

// first define actor
struct Xxx;
impl Actor for Xxx {}


// then define message
#[xtor::message(result = "i32")]
#[derive(Debug)]
struct Yyy;

// then impl the handler
#[async_trait]
impl Handler<Yyy> for Xxx {
    async fn handle(&self, _ctx: &Context, msg: Yyy) -> Result<i32> {
        println!("{:?} received",&msg);
        Ok(0)
    }
}

// main will finish when all actors died out.
#[xtor::main]
async fn main(){
    let x = Xxx;
    let x_address = x.spawn().await.unwrap();
    x_address.call::<Xxx, Yyy>(Yyy).await.unwrap();
}
```


## project structure
- `src/actor/*` for pure async actor implementation
- `src/sync_actor/*` for single threaded sync actor implementation
- `src/utils/*` for utilities both trait and default implementation such as
  - `Broker`
  - `Service`
  - `Supervisor`


## References
- [Actix](https://github.com/actix/actix)
- [Xactor](https://github.dev/sunli829/xactor)
- [Tokio](https://github.com/tokio-rs/tokio)
