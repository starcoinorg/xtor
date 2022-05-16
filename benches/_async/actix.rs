use actix::Actor;

struct CounterActor {
    count: usize,
}

impl actix::Actor for CounterActor {
    type Context = actix::Context<Self>;
}

struct SleepAddOne;

impl actix::Message for SleepAddOne {
    type Result = ();
}

impl actix::Handler<SleepAddOne> for CounterActor {
    type Result = ();

    fn handle(&mut self, _msg: SleepAddOne, _ctx: &mut actix::Context<Self>) -> Self::Result {
        // std::thread::sleep(std::time::Duration::from_millis(10));
        self.count += 1;
    }
}

pub fn run(spec: super::Spec) {
    let runtime = actix::System::new();
    let addr = CounterActor { count: 0 };
    runtime.block_on(async {
        let addr = addr.start();

        let mut futures = vec![];
        for _ in 0..spec.parallel {
            let addr = addr.clone();
            futures.push(async move {
                for _ in 0..spec.number {
                    addr.send(SleepAddOne).await.unwrap();
                }
            });
        }

        futures::future::join_all(futures).await;
    })
}
