use xactor::{Actor, Handler};

struct CounterActor {
    count: usize,
}

impl xactor::Actor for CounterActor {}

#[xactor::message(result = "()")]
struct SleepAddOne;

#[async_trait::async_trait]
impl Handler<SleepAddOne> for CounterActor {
    async fn handle(&mut self, _ctx: &mut xactor::Context<Self>, _msg: SleepAddOne) -> () {
        // tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        self.count += 1;
    }
}

pub fn run(spec: super::Spec) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(spec.parallel)
        .build()
        .unwrap();
    runtime.block_on(async {
        let addr = CounterActor { count: 0 }.start().await.unwrap();

        let mut futures = vec![];
        for _ in 0..spec.parallel {
            let addr = addr.clone();
            futures.push(async move {
                for _ in 0..spec.number {
                    addr.call(SleepAddOne).await.unwrap();
                }
            });
        }

        futures::future::join_all(futures).await;
    });
}
