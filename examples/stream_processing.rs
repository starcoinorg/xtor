use std::sync::atomic::AtomicUsize;

use futures::{try_join, Stream};

use xtor::{
    actor::{context::Context, message::Handler, runner::Actor},
    broker::Subscribe,
    utils::default_broker::{DefaultBroker, StreamBroker},
};

#[derive(Debug, Clone)]
#[xtor::message(result = "()")]
struct Number(usize);

struct RangeStream {
    current: AtomicUsize,
    end: usize,
}

impl Stream for RangeStream {
    type Item = Number;
    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let current = self
            .current
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if current < self.end {
            std::task::Poll::Ready(Some(Number(current)))
        } else {
            std::task::Poll::Ready(None)
        }
    }
}

struct EvenSubscriptor;
#[async_trait::async_trait]
impl Actor for EvenSubscriptor {
    async fn on_stop(&self, _ctx: &Context) {
        println!("EvenSubscriptor stopped");
    }
}
#[async_trait::async_trait]
impl Handler<Number> for EvenSubscriptor {
    async fn handle(&self, _ctx: &Context, msg: Number) -> anyhow::Result<()> {
        if msg.0 % 2 == 0 {
            println!("Even: {:?} received", &msg);
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
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
    async fn handle(&self, _ctx: &Context, msg: Number) -> anyhow::Result<()> {
        if msg.0 > 5 {
            println!("Big: {:?} received", &msg);
        }
        Ok(())
    }
}

#[xtor::main]
async fn main() -> anyhow::Result<()> {
    let stream = RangeStream {
        current: AtomicUsize::new(0),
        end: 10,
    };

    // create actor
    let (even, big) = try_join!(EvenSubscriptor.spawn(), BigNumberSubscriptor.spawn())?;

    // create stream broker
    let (broker, h) = StreamBroker(stream).spawn().await?;

    // subscribe to broker
    try_join!(
        broker.call::<DefaultBroker<Number>, Subscribe<Number>>(
            Subscribe::from_addr::<EvenSubscriptor>(&even).await,
        ),
        broker.call::<DefaultBroker<Number>, Subscribe<Number>>(
            Subscribe::from_addr::<BigNumberSubscriptor>(&big).await,
        )
    )?;

    h.await?.map_err(|e| e)
}
