use super::*;

#[crate::test]
async fn test_single_actor_single_message_blocked() {
    let actor = TestActor.spawn().await.unwrap();
    for i in 0..100 {
        let result = actor
            .call::<TestActor, TestAdd1Message>(TestAdd1Message(i))
            .await
            .unwrap();
        assert_eq!(result, i + 1);
    }
}

#[crate::test]
async fn test_two_actor_two_message_unblocked() {
    let a1 = TestActor.spawn().await.unwrap();
    let a2 = TestActor.spawn().await.unwrap();
    let start = std::time::Instant::now();
    let res1 = a1
        .call_unblock::<TestActor, TestSleepAdd1Message>(TestSleepAdd1Message(2000))
        .await;
    let res2 = a2
        .call_unblock::<TestActor, TestSleepAdd1Message>(TestSleepAdd1Message(1000))
        .await;
    let res = res2.await.unwrap().unwrap();
    assert_eq!(res, 1001);
    assert!(std::time::Instant::now() - start < std::time::Duration::from_millis(2000));
    assert_eq!(res1.await.unwrap().unwrap(), 2001);
}

#[crate::test]
async fn test_single_actor_single_message_blocked_error() {
    let actor = TestActor.spawn().await.unwrap();
    let result = actor
        .call::<TestActor, TestResultMessage<i32>>(TestResultMessage(Err(anyhow::anyhow!("error"))))
        .await;
    assert!(result.is_err());
}

#[crate::test]
async fn test_single_actor_single_message_unblocked_error() {
    let actor = TestActor.spawn().await.unwrap();
    let result = actor
        .call_unblock::<TestActor, TestResultMessage<i32>>(TestResultMessage(Err(anyhow::anyhow!(
            "error"
        ))))
        .await;
    // actor will stop as soon as the message is received
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    // and the channel for the message is closed
    assert!(result.await.is_err());
}

#[crate::test]
async fn test_single_actor_single_message_timeout_ontime() {
    let actor = TestActor.spawn().await.unwrap();
    let result = actor
        .call_timeout::<TestActor, TestSleepAdd1Message>(
            TestSleepAdd1Message(1000),
            std::time::Duration::from_millis(1500),
        )
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().unwrap(), 1001);
}

#[crate::test]
async fn test_single_actor_single_message_timeout_delayed() {
    let actor = TestActor.spawn().await.unwrap();
    let result = actor
        .call_timeout::<TestActor, TestSleepAdd1Message>(
            TestSleepAdd1Message(1500),
            std::time::Duration::from_millis(1000),
        )
        .await;
    assert!(result.unwrap().is_none());
}

#[crate::test]
async fn test_message_to_dead_actor() {
    let actor = TestActor.spawn().await.unwrap();
    let a = actor.clone();
    actor.stop(Ok(()));
    let result = a
        .call::<TestActor, TestAdd1Message>(TestAdd1Message(1))
        .await;
    assert!(result.is_err());
}
