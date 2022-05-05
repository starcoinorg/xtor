use std::sync::atomic::AtomicUsize;

use xtor::actor::{runner::Actor, context::Context, message::Handler};

struct CounterActor {
    count: AtomicUsize,
}

impl Actor for CounterActor {}

#[xtor::message(result = "()")]
struct SleepAddOne;

#[async_trait::async_trait]
impl Handler<SleepAddOne> for CounterActor {
    async fn handle(&self, _ctx: &Context, _msg: SleepAddOne) -> anyhow::Result<()> {
        // tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
}

pub fn run(spec: super::Spec) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(spec.parallel)
        .build()
        .unwrap();
    runtime.block_on(async {
        let addr = CounterActor {
            count: AtomicUsize::new(0),
        }
        .spawn()
        .await
        .unwrap();

        let mut futures = vec![];
        for _ in 0..spec.parallel {
            let addr = addr.clone();
            futures.push(async move {
                for _ in 0..spec.number {
                    addr.call::<CounterActor, SleepAddOne>(SleepAddOne)
                        .await
                        .unwrap();
                }
            });
        }

        futures::future::join_all(futures).await;
    });
}
