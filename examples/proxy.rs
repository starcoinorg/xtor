use anyhow::Result;
use std::sync::atomic::AtomicUsize;
use xtor::actor::{context::Context, message::Handler, runner::Actor};

#[xtor::message(result = "()")]
struct AddOne;

#[xtor::message(result = "()")]
struct Print;

struct CounterActor {
    counter: AtomicUsize,
}

#[async_trait::async_trait]
impl Actor for CounterActor {
    async fn on_stop(&self, ctx: &Context) {
        println!("{} stop", self.get_name_or_id_string(ctx).await);
    }
}

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
    async fn handle(&self, _ctx: &xtor::actor::context::Context, _msg: Print) -> Result<()> {
        println!("{}", self.counter.load(std::sync::atomic::Ordering::SeqCst));
        Ok(())
    }
}

#[xtor::main]
async fn main() -> anyhow::Result<()> {
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
