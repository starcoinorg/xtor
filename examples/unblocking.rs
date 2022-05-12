use futures::{join, try_join};
use rand::{prelude::StdRng, Rng, SeedableRng};

use xtor::actor::{context::Context, message::Handler, runner::Actor};

struct Oracle;

impl Actor for Oracle {}

#[xtor::message(result = "u32")]
struct GetOracleNumber;

#[async_trait::async_trait]
impl Handler<GetOracleNumber> for Oracle {
    async fn handle(&self, _ctx: &Context, _msg: GetOracleNumber) -> anyhow::Result<u32> {
        let mut rng = StdRng::from_entropy();
        let sleep_time = rng.gen_range(0..1000) as u32;
        tokio::time::sleep(std::time::Duration::from_millis(sleep_time as u64)).await;
        Ok(sleep_time)
    }
}

#[xtor::main]
async fn main() -> anyhow::Result<()> {
    let (oracle1, oracle2, oracle3) = try_join!(Oracle.spawn(), Oracle.spawn(), Oracle.spawn())?;
    let start = std::time::Instant::now();
    while start.elapsed().as_secs() < 10 {
        let (oracle1_number, oracle2_number, oracle3_number) = join!(
            oracle1.call_unblock::<Oracle, GetOracleNumber>(GetOracleNumber),
            oracle2.call_unblock::<Oracle, GetOracleNumber>(GetOracleNumber),
            oracle3.call_unblock::<Oracle, GetOracleNumber>(GetOracleNumber)
        );
        tokio::select! {
            o1 = oracle1_number => {
                println!("oracle1: {}", o1??);
            },
            o2 = oracle2_number => {
                println!("oracle2: {}", o2??);
            },
            o3 = oracle3_number => {
                println!("oracle3: {}", o3??);
            },
            _ = tokio::time::sleep(std::time::Duration::from_millis(1000)) => {
                println!("no oracle in this second");
            }
        }
    }
    Ok(())
}
