use std::sync::atomic::AtomicUsize;

use futures::{Stream, StreamExt};
use tokio::task::JoinHandle;
use xtor::{
    actor::{context::Context, message::Handler, runner::Actor},
    utils::broker::{DefaultBroker, Publish, Subscribe},
    Addr, Message,
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

pub struct StreamBroker<S: Stream<Item = I> + Sync + Send + 'static, I: Message + Clone>(pub S);
impl<S: Stream<Item = I> + Sync + Send + 'static, I: Message + Clone> Actor for StreamBroker<S, I> {}

impl<S: Stream<Item = I> + Sync + Send + 'static + Unpin, I: Message + Sync + Clone>
    StreamBroker<S, I>
{
    pub async fn spawn(mut self) -> anyhow::Result<(Addr, JoinHandle<anyhow::Result<()>>)> {
        let broker = DefaultBroker::<I>::new().spawn().await?;
        let broker_a = broker.clone();
        Ok((
            broker,
            tokio::spawn(async move {
                while let Some(msg) = self.0.next().await {
                    broker_a
                        .call::<DefaultBroker<I>, Publish<I>>(Publish(msg))
                        .await?;
                }
                Ok(())
            }),
        ))
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
async fn main() {
    let stream = RangeStream {
        current: AtomicUsize::new(0),
        end: 10,
    };

    let even = EvenSubscriptor.spawn().await.unwrap();
    let big = BigNumberSubscriptor.spawn().await.unwrap();

    let (broker, h) = StreamBroker(stream).spawn().await.unwrap();

    broker
        .call::<DefaultBroker<Number>, Subscribe<Number>>(
            Subscribe::from_addr::<EvenSubscriptor>(even.clone()).await,
        )
        .await
        .unwrap();

    broker
        .call::<DefaultBroker<Number>, Subscribe<Number>>(
            Subscribe::from_addr::<BigNumberSubscriptor>(big.clone()).await,
        )
        .await
        .unwrap();
    h.await.unwrap().unwrap();
}
