use std::sync::atomic::AtomicUsize;

use crate::{
    broker::{Publish, Subscribe},
    utils::default_broker::DefaultBroker,
};

use super::*;

struct TestState(AtomicUsize);
impl Default for TestState {
    fn default() -> Self {
        Self(AtomicUsize::new(0))
    }
}
impl Actor for TestState {}

#[async_trait::async_trait]
impl Handler<TestAdd1Message> for TestState {
    async fn handle(&self, _ctx: &Context, _msg: TestAdd1Message) -> anyhow::Result<i32> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(0)
    }
}

#[crate::message(result = "usize")]
struct Get;

#[async_trait::async_trait]
impl Handler<Get> for TestState {
    async fn handle(&self, _ctx: &Context, _msg: Get) -> anyhow::Result<usize> {
        Ok(self.0.load(std::sync::atomic::Ordering::SeqCst))
    }
}

#[crate::test]
async fn test_all_message_arrive() {
    let broker = DefaultBroker::<TestAdd1Message>::new()
        .spawn()
        .await
        .unwrap();
    let subscribe_proxy = broker
        .proxy::<DefaultBroker<TestAdd1Message>, Subscribe<TestAdd1Message>>()
        .await;
    let a1 = TestState::default().spawn().await.unwrap();
    let a2 = TestState::default().spawn().await.unwrap();
    let a3 = TestState::default().spawn().await.unwrap();

    subscribe_proxy
        .call(Subscribe::from_addr::<TestState>(&a1).await)
        .await
        .unwrap();
    subscribe_proxy
        .call(Subscribe::from_addr::<TestState>(&a2).await)
        .await
        .unwrap();
    subscribe_proxy
        .call(Subscribe::from_addr::<TestState>(&a3).await)
        .await
        .unwrap();

    broker
        .call::<DefaultBroker<TestAdd1Message>, Publish<TestAdd1Message>>(Publish(TestAdd1Message(
            1,
        )))
        .await
        .unwrap();

    assert_eq!(a1.call::<TestState, Get>(Get).await.unwrap(), 1);
    assert_eq!(a2.call::<TestState, Get>(Get).await.unwrap(), 1);
    assert_eq!(a3.call::<TestState, Get>(Get).await.unwrap(), 1);
}
