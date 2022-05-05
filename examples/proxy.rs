use anyhow::Result;
use std::sync::atomic::AtomicUsize;
use xtor::actor::{
    context::Context,
    message::Handler,
    runner::{Actor, ActorRunner},
};

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
        println!("{} stop", self.get_name(ctx).await.unwrap());
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
async fn main() {
    let counter = CounterActor {
        counter: AtomicUsize::new(0),
    };
    let addr = ActorRunner::new().run(counter).await.unwrap();
    addr.set_name("Counter Actor").await;
    let proxy = addr.proxy::<CounterActor, AddOne>().await;
    tokio::task::spawn(async move {
        loop {
            proxy.call(AddOne).await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    });
    let a = addr.clone();
    tokio::task::spawn(async move {
        loop {
            a.call::<CounterActor, Print>(Print).await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    });

    let mut buffer = String::new();
    std::io::stdin().read_line(&mut buffer).unwrap();
    addr.stop(Ok(()));
}
