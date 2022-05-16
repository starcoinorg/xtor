mod test_actor;
mod test_broker;
mod test_message;
mod test_proxy;
mod test_supervisor;

use crate::{self as xtor, Actor, Context, Handler, Message};

pub(crate) struct TestActor;
impl Actor for TestActor {}

// i + 1
#[derive(Clone, Copy)]
#[crate::message(result = "i32")]
pub(crate) struct TestAdd1Message(i32);

#[async_trait::async_trait]
impl Handler<TestAdd1Message> for TestActor {
    async fn handle(&self, _ctx: &Context, msg: TestAdd1Message) -> anyhow::Result<i32> {
        println!("{:?} received", &msg.0);
        Ok(msg.0 + 1)
    }
}

// sleep(i) => i + 1
#[crate::message(result = "i32")]
pub(crate) struct TestSleepAdd1Message(i32);

#[async_trait::async_trait]
impl Handler<TestSleepAdd1Message> for TestActor {
    async fn handle(&self, _ctx: &Context, msg: TestSleepAdd1Message) -> anyhow::Result<i32> {
        println!("{:?} received", &msg.0);
        tokio::time::sleep(std::time::Duration::from_millis(msg.0 as _)).await;
        Ok(msg.0 + 1)
    }
}

// ok(res) => ok(res)
// err(e) => err(e) + actor stop
struct TestResultMessage<T>(anyhow::Result<T>);

impl<T: 'static + Send> Message for TestResultMessage<T> {
    type Result = T;
}

#[async_trait::async_trait]
impl<T: 'static + Send> Handler<TestResultMessage<T>> for TestActor {
    async fn handle(&self, _ctx: &Context, msg: TestResultMessage<T>) -> anyhow::Result<T> {
        Ok(msg.0?)
    }
}
