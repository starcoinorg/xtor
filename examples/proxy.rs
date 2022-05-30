use std::sync::atomic::AtomicUsize;

use anyhow::Result;
use tracing::info;
use xtor::actor::{message::Handler, runner::Actor};

#[xtor::message(result = "()")]
struct AddOne;

#[derive(Debug)]
#[xtor::message(result = "()")]
struct Print;

struct CounterActor {
    counter: AtomicUsize,
}

#[async_trait::async_trait]
impl Actor for CounterActor {}

#[async_trait::async_trait]
impl Handler<AddOne> for CounterActor {
    async fn handle(&self, _ctx: &xtor::actor::context::Context, _msg: AddOne) -> Result<()> {
        self.counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
}

#[async_trait::async_trait]
impl Handler<Print> for CounterActor {
    #[tracing::instrument(
        skip(self,_ctx),
        name = "CounterActor::Print",
        fields(addr = self.get_name_or_id_string(_ctx).as_str())
    )]
    async fn handle(&self, _ctx: &xtor::actor::context::Context, _msg: Print) -> Result<()> {
        info!("{}", self.counter.load(std::sync::atomic::Ordering::SeqCst));
        Ok(())
    }
}

#[xtor::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // create actor
    let addr = CounterActor {
        counter: AtomicUsize::new(0),
    }
    .spawn()
    .await?;
    addr.set_name("Counter Actor").await;

    // create proxy
    let proxy = addr.proxy::<CounterActor, AddOne>().await;
    tokio::task::spawn(async move {
        let start = std::time::Instant::now();
        while start.elapsed().as_secs() < 10 {
            proxy.call(AddOne).await.expect("fail to call");
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    });

    tokio::task::spawn(async move {
        let start = std::time::Instant::now();
        while start.elapsed().as_secs() < 10 {
            addr.call::<CounterActor, Print>(Print)
                .await
                .expect("fail to call");
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    });

    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    Ok(())
}
