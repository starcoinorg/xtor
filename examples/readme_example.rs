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
