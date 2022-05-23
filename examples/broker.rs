use anyhow::Result;
use futures::try_join;
use tracing::info;
use xtor::{
    actor::{context::Context, message::Handler, runner::Actor},
    broker::{Publish, Subscribe},
    utils::default_broker::DefaultBroker,
};

struct EvenSubscriptor;

#[async_trait::async_trait]
impl Actor for EvenSubscriptor {}
#[async_trait::async_trait]
impl Handler<Number> for EvenSubscriptor {
    #[tracing::instrument(
        skip(self,_ctx),
        name = "EvenSubscriptor::Number",
        fields(addr = self.get_name_or_id_string(_ctx).as_str())
    )]
    async fn handle(&self, _ctx: &Context, msg: Number) -> Result<()> {
        if msg.0 % 2 == 0 {
            info!("Even: {:?} received", &msg);
        }
        Ok(())
    }
}
struct OddSubscriptor;
#[async_trait::async_trait]
impl Actor for OddSubscriptor {}
#[async_trait::async_trait]
impl Handler<Number> for OddSubscriptor {
    #[tracing::instrument(
        skip(self,_ctx),
        name = "OddSubscriptor::Number",
        fields(addr = self.get_name_or_id_string(_ctx).as_str())
    )]
    async fn handle(&self, _ctx: &Context, msg: Number) -> Result<()> {
        if msg.0 % 2 != 0 {
            info!("Odd: {:?} received", &msg);
        }
        Ok(())
    }
}

struct BigNumberSubscriptor;
#[async_trait::async_trait]
impl Actor for BigNumberSubscriptor {}

#[async_trait::async_trait]
impl Handler<Number> for BigNumberSubscriptor {
    #[tracing::instrument(
        skip(self,_ctx),
        name = "BigNumberSubscriptor::Number",
        fields(addr = self.get_name_or_id_string(_ctx).as_str())
    )]
    async fn handle(&self, _ctx: &Context, msg: Number) -> Result<()> {
        if msg.0 > 5 {
            info!("Big: {:?} received", &msg);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy)]
#[xtor::message(result = "()")]
struct Number(u64);

#[xtor::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // create actor
    let (even, odd, big) = try_join!(
        EvenSubscriptor.spawn(),
        OddSubscriptor.spawn(),
        BigNumberSubscriptor.spawn(),
    )?;

    // create broker and proxy
    let broker = DefaultBroker::<Number>::new().spawn().await?;
    let subscribe_proxy = broker
        .proxy::<DefaultBroker<Number>, Subscribe<Number>>()
        .await;

    // subscribe to broker
    try_join!(
        subscribe_proxy.call(Subscribe::from_addr::<EvenSubscriptor>(&even).await),
        subscribe_proxy.call(Subscribe::from_addr::<OddSubscriptor>(&odd).await),
        subscribe_proxy.call(Subscribe::from_addr::<BigNumberSubscriptor>(&big).await),
    )?;

    let broker_clone = broker.clone();
    let h = tokio::task::spawn(async move {
        let broker_publish_proxy = broker_clone
            .proxy::<DefaultBroker<Number>, Publish<Number>>()
            .await;
        for i in 0..10 {
            info!("Publish: {}", i);
            broker_publish_proxy
                .call(Publish(Number(i)))
                .await
                .expect("publish failed");
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    });

    h.await.map_err(|e| e.into())
}
