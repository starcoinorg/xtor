use rand::{prelude::StdRng, Rng, SeedableRng};
use tracing::{info, warn};
use xtor::actor::{context::Context, message::Handler, runner::Actor};

struct Oracle;

impl Actor for Oracle {}

#[derive(Debug)]
#[xtor::message(result = "u32")]
struct GetOracleNumber;

#[async_trait::async_trait]
impl Handler<GetOracleNumber> for Oracle {
    #[tracing::instrument(
        skip(self, _ctx),
        name = "Oracle::GetOracleNumber",
        fields(addr = self.get_name_or_id_string(_ctx).as_str())
    )]
    async fn handle(&self, _ctx: &Context, _msg: GetOracleNumber) -> anyhow::Result<u32> {
        let mut rng = StdRng::from_entropy();
        let sleep_time = rng.gen_range(0..1000) as u32;
        tokio::time::sleep(std::time::Duration::from_millis(sleep_time as u64)).await;
        info!("oracle: {}", sleep_time);
        Ok(sleep_time)
    }
}

#[xtor::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let oracle1 = Oracle.spawn().await?;
    let start = std::time::Instant::now();
    while start.elapsed().as_secs() < 10 {
        let oracle1_number = oracle1
            .call_timeout::<Oracle, GetOracleNumber>(
                GetOracleNumber,
                std::time::Duration::from_millis(500),
            )
            .await?;
        match oracle1_number {
            Some(o1) => {
                info!("oracle1: {}", o1);
            }
            None => {
                warn!("no oracle in 500 ms");
            }
        }
    }
    Ok(())
}
