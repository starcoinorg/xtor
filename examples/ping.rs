use anyhow::Result;
use futures::join;
use std::{mem::MaybeUninit, sync::atomic::AtomicUsize};
use xtor::actor::{
    actor::{Actor, ActorRunner},
    addr::Addr,
    context::Context,
    message::Handler,
};

#[xtor::message(result = "isize")]
struct Ping(isize);

#[xtor::message(result = "()")]
struct SetPingAddress(Addr);
struct PingActor {
    sleeper: isize,
    n: isize,
    counter: AtomicUsize,
    ping_address: Addr,
}

#[async_trait::async_trait]
impl Actor for PingActor {
    async fn on_stop(&self, ctx: &Context) {
        println!("{} stop", self.get_name(ctx).await.unwrap());
    }
}

#[async_trait::async_trait]
impl Handler<Ping> for PingActor {
    async fn handle(&self, ctx: &Context, msg: Ping) -> Result<isize> {
        println!(
            "{}: {}",
            self.get_name(ctx).await.unwrap_or_default(),
            msg.0
        );
        let addr = self.ping_address.clone();
        let n = self.n;
        let _ = tokio::task::spawn(async move {
            addr.call_unblock::<PingActor, Ping>(Ping(msg.0 + n)).await
        });
        tokio::time::sleep(std::time::Duration::from_millis(self.sleeper as u64)).await;
        self.counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(msg.0 + 1)
    }
}
#[async_trait::async_trait]
impl Handler<SetPingAddress> for PingActor {
    async fn handle(&self, _ctx: &Context, msg: SetPingAddress) -> Result<()> {
        unsafe { std::ptr::write(&self.ping_address as *const _ as *mut _, Some(msg.0)) }
        Ok(())
    }
}

#[xtor::main]
async fn main() {
    println!("Ping Pong!\nPress any key to exit");
    let ping1 = PingActor {
        sleeper: 100,
        n: 1,
        counter: AtomicUsize::new(0),
        ping_address: unsafe { MaybeUninit::uninit().assume_init() },
    };
    let ping2 = PingActor {
        sleeper: 200,
        n: -1,
        counter: AtomicUsize::new(0),
        ping_address: unsafe { MaybeUninit::uninit().assume_init() },
    };
    let p1 = ActorRunner::new().run(ping1).await.unwrap();
    let p2 = ActorRunner::new().run(ping2).await.unwrap();
    join! {p1.set_name("ping"), p2.set_name("pong")};
    let _ = join! {
        p1.call::<PingActor, SetPingAddress>(SetPingAddress(p2.clone())),
        p2.call::<PingActor, SetPingAddress>(SetPingAddress(p1.clone()))
    };

    p1.call::<PingActor, Ping>(Ping(0)).await.unwrap();

    let mut buffer = String::new();
    std::io::stdin().read_line(&mut buffer).unwrap();
    p1.clone().stop(Ok(()));
    p2.clone().stop(Ok(()));
}
