use anyhow::Result;
use xtor::{
    actor::{
        actor::{Actor, ActorRunner},
        context::Context,
        message::Handler,
    },
    utils::broker::{Broker, DefaultBroker, Publish, Subscribe},
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
async fn main() {
    let even = EvenSubscriptor.spawn().await.unwrap();
    let odd = OddSubscriptor.spawn().await.unwrap();
    let big = BigNumberSubscriptor.spawn().await.unwrap();

    let broker = DefaultBroker::<Number>::new().spawn().await.unwrap();
    broker
        .call::<DefaultBroker<Number>, Subscribe<Number>>(
            Subscribe::from_addr::<EvenSubscriptor>(even.clone()).await,
        )
        .await
        .unwrap();
    broker
        .call::<DefaultBroker<Number>, Subscribe<Number>>(
            Subscribe::from_addr::<OddSubscriptor>(odd.clone()).await,
        )
        .await
        .unwrap();
    broker
        .call::<DefaultBroker<Number>, Subscribe<Number>>(
            Subscribe::from_addr::<BigNumberSubscriptor>(big.clone()).await,
        )
        .await
        .unwrap();

    let broker_clone = broker.clone();
    let h = tokio::task::spawn(async move {
        for i in 0..10 {
            println!("\nPublish: {}", i);
            broker_clone
                .call::<DefaultBroker<Number>, Publish<Number>>(Publish(Number(i)))
                .await
                .unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    });
    h.await;
}
