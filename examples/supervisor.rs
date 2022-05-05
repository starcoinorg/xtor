// An actor called human is a human.
// a message eat could eat a food.
// food has two types: normal and poisoned.
// normal food is good for human.
// poisoned food will kill human.
// food_hospital is a supervisor that can respawn(lol) a human.

// another thing is called virus.
// virus is contagious.
// virus can infect human.
// virus_hospital will respawn all human if even only one human is dead.
// virus_hospital is not very clever, for every desease it will think that it is a virus.

use std::sync::atomic::AtomicBool;

use rand::{prelude::StdRng, Rng, SeedableRng};
use xtor::{
    actor::{
        runner::{Actor, ActorRestart},
        addr::WeakAddr,
        context::Context,
        message::Handler,
        supervisor::Supervise,
    },
    utils::default_supervisor::DefaultSupervisor,
};

struct Human(AtomicBool);
impl Default for Human {
    fn default() -> Self {
        Human(AtomicBool::new(false))
    }
}
#[async_trait::async_trait]
impl Actor for Human {}

#[async_trait::async_trait]
impl ActorRestart for Human {
    async fn on_restart(&self, addr: &WeakAddr) {
        println!(
            "\thospital saved {}",
            addr.upgrade()
                .unwrap()
                .get_name()
                .await
                .unwrap_or(format!("actor {}", addr.id))
        );
        self.0.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

#[derive(Debug, Clone)]
enum Food {
    Normal(&'static str),
    Poisoned(&'static str),
}

#[xtor::message(result = "()")]
struct Eat(Food);

#[async_trait::async_trait]
impl Handler<Eat> for Human {
    async fn handle(&self, ctx: &Context, msg: Eat) -> anyhow::Result<()> {
        match msg.0 {
            Food::Normal(food) => {
                println!(
                    "{} eat {:?}",
                    self.get_name(ctx)
                        .await
                        .unwrap_or(format!("actor {}", ctx.id)),
                    food
                );
                Ok(())
            }
            Food::Poisoned(food) => {
                println!(
                    "{} eat {:?}, and dead",
                    self.get_name(ctx)
                        .await
                        .unwrap_or(format!("actor {}", ctx.id)),
                    food
                );
                self.0.store(true, std::sync::atomic::Ordering::SeqCst);
                Err(anyhow::anyhow!("dead"))
            }
        }
    }
}

#[xtor::message(result = "()")]
struct Virus;

#[async_trait::async_trait]
impl Handler<Virus> for Human {
    async fn handle(&self, ctx: &Context, _msg: Virus) -> anyhow::Result<()> {
        println!(
            "{} infected, and dead",
            self.get_name(ctx)
                .await
                .unwrap_or(format!("actor {}", ctx.id))
        );
        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
        Err(anyhow::anyhow!("dead"))
    }
}

#[xtor::main]
async fn main() {
    let system_running_duration = 10;
    let normal_hospital = DefaultSupervisor::new(
        xtor::utils::default_supervisor::DefaultSupervisorRestartStrategy::OneForOne,
    )
    .spawn()
    .await
    .unwrap();

    let virus_hospital = DefaultSupervisor::new(
        xtor::utils::default_supervisor::DefaultSupervisorRestartStrategy::OneForAll,
    )
    .spawn()
    .await
    .unwrap();

    let alice = Human::default()
        .spawn_supervised::<DefaultSupervisor>(&normal_hospital)
        .await
        .unwrap();
    let bob = Human::default()
        .spawn_supervised::<DefaultSupervisor>(&normal_hospital)
        .await
        .unwrap();
    let carl = Human::default()
        .spawn_supervised::<DefaultSupervisor>(&normal_hospital)
        .await
        .unwrap();
    let david = Human::default()
        .spawn_supervised::<DefaultSupervisor>(&normal_hospital)
        .await
        .unwrap();

    alice.set_name("Alice").await;
    bob.set_name("Bob").await;
    carl.set_name("Carl").await;
    david.set_name("David").await;

    // register to normal hospital
    {
        normal_hospital
            .call::<DefaultSupervisor, Supervise>(Supervise(alice.clone()))
            .await
            .unwrap();
        normal_hospital
            .call::<DefaultSupervisor, Supervise>(Supervise(bob.clone()))
            .await
            .unwrap();
        normal_hospital
            .call::<DefaultSupervisor, Supervise>(Supervise(carl.clone()))
            .await
            .unwrap();
        normal_hospital
            .call::<DefaultSupervisor, Supervise>(Supervise(david.clone()))
            .await
            .unwrap();
    }

    // register to virus hospital
    {
        virus_hospital
            .call::<DefaultSupervisor, Supervise>(Supervise(alice.clone()))
            .await
            .unwrap();

        // bob said that he has a very strong body
        // thus virus could not come out so he decided to only paying the normal hospital

        virus_hospital
            .call::<DefaultSupervisor, Supervise>(Supervise(carl.clone()))
            .await
            .unwrap();
        virus_hospital
            .call::<DefaultSupervisor, Supervise>(Supervise(david.clone()))
            .await
            .unwrap();
    }

    let humans = [alice.clone(), bob.clone(), carl.clone(), david.clone()];
    let food_feeder = tokio::task::spawn(async move {
        let foods = [
            Food::Normal("apple"),
            Food::Normal("banana"),
            Food::Normal("orange"),
            Food::Poisoned("red mushroom"),
        ];
        let start = std::time::Instant::now();
        while start.elapsed().as_secs() < system_running_duration {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            let mut rng = StdRng::from_entropy();
            let selected_human = humans.get(rng.gen_range(0..humans.len())).unwrap();
            let selected_food = foods.get(rng.gen_range(0..foods.len())).unwrap().clone();
            let _ = selected_human.call::<Human, Eat>(Eat(selected_food)).await;
        }
    });

    let humans = [alice.clone(), bob.clone()];
    let virus_inflecter = tokio::task::spawn(async move {
        let start = std::time::Instant::now();
        while start.elapsed().as_secs() < system_running_duration {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            let mut rng = StdRng::from_entropy();
            let selected_human = humans.get(rng.gen_range(0..humans.len())).unwrap();
            let _ = selected_human.call::<Human, Virus>(Virus).await;
        }
    });

    food_feeder.await.unwrap();
    virus_inflecter.await.unwrap();
}
