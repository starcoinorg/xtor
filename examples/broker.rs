use anyhow::Result;
use futures::try_join;
use xtor::{
    actor::{context::Context, message::Handler, runner::Actor},
    broker::{Publish, Subscribe},
    utils::default_broker::DefaultBroker,
};

struct EvenSubscriptor;

#[async_trait::async_trait]
impl Actor for EvenSubscriptor {
    async fn on_stop(&self, _ctx: &Context) {
        println!("EvenSubscriptor stopped");
    }
}
#[async_trait::async_trait]
impl Handler<Number> for EvenSubscriptor {
    async fn handle(&self, _ctx: &Context, msg: Number) -> Result<()> {
        if msg.0 % 2 == 0 {
            println!("Even: {:?} received", &msg);
        }
        Ok(())
    }
}
struct OddSubscriptor;
#[async_trait::async_trait]
impl Actor for OddSubscriptor {
    async fn on_stop(&self, _ctx: &Context) {
        println!("OddSubscriptor stopped");
    }
}
#[async_trait::async_trait]
impl Handler<Number> for OddSubscriptor {
    async fn handle(&self, _ctx: &Context, msg: Number) -> Result<()> {
        if msg.0 % 2 != 0 {
            println!("Odd: {:?} received", &msg);
        }
        Ok(())
    }
}

struct BigNumberSubscriptor;
#[async_trait::async_trait]
impl Actor for BigNumberSubscriptor {
    async fn on_stop(&self, _ctx: &Context) {
        println!("BigNumberSubscriptor stopped");
    }
}
#[async_trait::async_trait]
impl Handler<Number> for BigNumberSubscriptor {
    async fn handle(&self, _ctx: &Context, msg: Number) -> Result<()> {
        if msg.0 > 5 {
            println!("Big: {:?} received", &msg);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy)]
#[xtor::message(result = "()")]
struct Number(u64);

#[xtor::main]
async fn main() -> anyhow::Result<()> {
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
        subscribe_proxy.call(Subscribe::from_addr::<EvenSubscriptor>(even.clone()).await),
        subscribe_proxy.call(Subscribe::from_addr::<OddSubscriptor>(odd.clone()).await),
        subscribe_proxy.call(Subscribe::from_addr::<BigNumberSubscriptor>(big.clone()).await),
    )?;

    let broker_clone = broker.clone();
    let h = tokio::task::spawn(async move {
        let broker_publish_proxy = broker_clone
            .proxy::<DefaultBroker<Number>, Publish<Number>>()
            .await;
        for i in 0..10 {
            println!("\nPublish: {}", i);
            broker_publish_proxy
                .call(Publish(Number(i)))
                .await
                .expect("publish failed");
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    });

    h.await.map_err(|e| e.into())
}
