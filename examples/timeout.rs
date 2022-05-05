use rand::{prelude::StdRng, Rng, SeedableRng};

use xtor::actor::{runner::Actor, context::Context, message::Handler};

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
async fn main() {
    let oracle1 = Oracle.spawn().await.unwrap();
    let start = std::time::Instant::now();
    while start.elapsed().as_secs() < 10 {
        let oracle1_number = oracle1
            .call_timeout::<Oracle, GetOracleNumber>(
                GetOracleNumber,
                std::time::Duration::from_millis(500),
            )
            .await
            .unwrap();
        match oracle1_number {
            Some(o1) => {
                println!("oracle1: {}", o1);
            }
            None => {
                println!("no oracle in 500 ms");
            }
        }
    }
}
