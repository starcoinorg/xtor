use anyhow::Result;
use futures::{future::join, join};
use std::{mem::MaybeUninit, sync::atomic::AtomicUsize};
use xtor::actor::{actor::Actor, addr::Addr, context::Context, message::Handler};

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
    let ping = PingActor {
        sleeper: 100,
        n: 1,
        counter: AtomicUsize::new(0),
        ping_address: unsafe { MaybeUninit::uninit().assume_init() },
    };
    let pong = PingActor {
        sleeper: 200,
        n: -1,
        counter: AtomicUsize::new(0),
        ping_address: unsafe { MaybeUninit::uninit().assume_init() },
    };
    let (ping_addr, pong_addr) = join(ping.spawn(), pong.spawn()).await;
    let (ping_addr, pong_addr) = (ping_addr.unwrap(), pong_addr.unwrap());
    join! {ping_addr.set_name("ping"), pong_addr.set_name("pong")};
    let _ = join! {
        ping_addr.call::<PingActor, SetPingAddress>(SetPingAddress(pong_addr.clone())),
        pong_addr.call::<PingActor, SetPingAddress>(SetPingAddress(ping_addr.clone()))
    };

    ping_addr.call::<PingActor, Ping>(Ping(0)).await.unwrap();

    let mut buffer = String::new();
    std::io::stdin().read_line(&mut buffer).unwrap();

    ping_addr.clone().stop(Ok(()));
    pong_addr.clone().stop(Ok(()));
}
