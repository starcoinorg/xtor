use anyhow::Result;
use futures::{join, try_join};
use once_cell::sync::OnceCell;
use std::sync::atomic::AtomicUsize;
use xtor::actor::{addr::WeakAddr, context::Context, message::Handler, runner::Actor};

#[xtor::message(result = "isize")]
struct Ping(isize);

#[xtor::message(result = "()")]
struct SetPingAddress(WeakAddr);
struct PingActor {
    sleeper: isize,
    n: isize,
    counter: AtomicUsize,
    ping_address: OnceCell<WeakAddr>,
}

#[async_trait::async_trait]
impl Actor for PingActor {}

#[async_trait::async_trait]
impl Handler<Ping> for PingActor {
    async fn handle(&self, ctx: &Context, msg: Ping) -> Result<isize> {
        println!("{}: {}", self.get_name_or_id_string(ctx).await, msg.0);
        match self.ping_address.get().expect("fail to get").upgrade() {
            Some(addr) => {
                let n = self.n;
                let _ = tokio::task::spawn(async move {
                    addr.call_unblock::<PingActor, Ping>(Ping(msg.0 + n))
                        .await
                        .await
                });
                tokio::time::sleep(std::time::Duration::from_millis(self.sleeper as u64)).await;
                self.counter
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(msg.0 + 1)
            }
            None => {
                ctx.stop();
                Ok(msg.0)
            }
        }
    }
}
#[async_trait::async_trait]
impl Handler<SetPingAddress> for PingActor {
    async fn handle(&self, _ctx: &Context, msg: SetPingAddress) -> Result<()> {
        self.ping_address.set(msg.0).expect("fail to set");
        Ok(())
    }
}

#[xtor::main]
async fn main() -> anyhow::Result<()> {
    // create actors
    let (ping_addr, pong_addr) = try_join!(
        PingActor {
            sleeper: 100,
            n: 1,
            counter: AtomicUsize::new(0),
            ping_address: OnceCell::new(),
        }
        .spawn(),
        PingActor {
            sleeper: 200,
            n: -1,
            counter: AtomicUsize::new(0),
            ping_address: OnceCell::new(),
        }
        .spawn(),
    )?;

    // set their name
    join!(ping_addr.set_name("ping"), pong_addr.set_name("pong"));

    // set their address to each other
    try_join!(
        ping_addr.call::<PingActor, SetPingAddress>(SetPingAddress(pong_addr.downgrade())),
        pong_addr.call::<PingActor, SetPingAddress>(SetPingAddress(ping_addr.downgrade())),
    )?;

    ping_addr.call::<PingActor, Ping>(Ping(0)).await?;
    tokio::time::sleep(std::time::Duration::from_millis(10000)).await;
    ping_addr.stop(Ok(()));
    pong_addr.stop(Ok(()));
    Ok(())
}
