use anyhow::Result;
use async_trait::async_trait;
use tracing::info;
use xtor::actor::{context::Context, message::Handler, runner::Actor};
// first define actor
struct HelloAector;
impl Actor for HelloAector {}

// then define message
#[xtor::message(result = "()")]
#[derive(Debug)]
struct Hello;

// then impl the handler
#[async_trait]
impl Handler<Hello> for HelloAector {
    #[tracing::instrument(
        skip(self,_ctx),
        name = "HelloAector::Hello",
        fields(addr = self.get_name_or_id_string(_ctx).as_str())
    )]
    async fn handle(&self, _ctx: &Context, msg: Hello) -> Result<()> {
        info!("{:?} received", &msg);
        Ok(())
    }
}

// main will finish when all actors died out.
#[xtor::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let hello_actor = HelloAector;
    let hello_actor_address = hello_actor.spawn().await?;
    hello_actor_address.call::<HelloAector, Hello>(Hello).await
}
