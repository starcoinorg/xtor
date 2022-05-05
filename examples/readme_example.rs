use anyhow::Result;
use async_trait::async_trait;
use xtor::actor::{context::Context, message::Handler, runner::Actor};

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
        println!("{:?} received", &msg);
        Ok(0)
    }
}

// main will finish when all actors died out.
#[xtor::main]
async fn main() {
    let x = Xxx;
    let x_address = x.spawn().await.unwrap();
    x_address.call::<Xxx, Yyy>(Yyy).await.unwrap();
}
