use super::*;

#[crate::test]
async fn test_single_proxy_single_message_blocked() {
    let actor = TestActor.spawn().await.unwrap();
    let proxy = actor.proxy::<TestActor, TestAdd1Message>().await;
    for i in 0..100 {
        let result = proxy.call(TestAdd1Message(i)).await.unwrap();
        assert_eq!(result, i + 1);
    }
}

#[crate::test]
async fn test_two_proxy_two_message_unblocked() {
    let a1 = TestActor.spawn().await.unwrap();
    let a2 = TestActor.spawn().await.unwrap();
    let p1 = a1.proxy::<TestActor, TestSleepAdd1Message>().await;
    let p2 = a2.proxy::<TestActor, TestSleepAdd1Message>().await;
    let start = std::time::Instant::now();
    let res1 = p1
        .call_unblock(TestSleepAdd1Message(2000))
        .await
        .await
        .unwrap();
    let res2 = p2
        .call_unblock(TestSleepAdd1Message(1000))
        .await
        .await
        .unwrap();
    let res = res2.await.unwrap().unwrap();
    assert_eq!(res, 1001);
    assert!(std::time::Instant::now() - start < std::time::Duration::from_millis(2000));
    assert_eq!(res1.await.unwrap().unwrap(), 2001);
}

#[crate::test]
async fn test_single_proxy_single_message_blocked_error() {
    let actor = TestActor.spawn().await.unwrap();
    let proxy = actor.proxy::<TestActor, TestResultMessage<i32>>().await;
    let result = proxy
        .call(TestResultMessage(Err(anyhow::anyhow!("error"))))
        .await;
    assert!(result.is_err());
    assert!(actor.await_stop().await.unwrap() == ());
}

#[crate::test]
async fn test_single_proxy_single_message_unblocked_error() {
    let actor = TestActor.spawn().await.unwrap();
    let proxy = actor.proxy::<TestActor, TestResultMessage<i32>>().await;
    let result = proxy
        .call_unblock(TestResultMessage(Err(anyhow::anyhow!("error"))))
        .await
        .await
        .unwrap();
    // actor will stop as soon as the message is received
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(actor.await_stop().await.unwrap() == ());
    // and the channel for the message is closed
    assert!(result.await.is_err());
}

#[crate::test]
async fn test_single_proxy_single_message_timeout_ontime() {
    let actor = TestActor.spawn().await.unwrap();
    let proxy = actor.proxy::<TestActor, TestSleepAdd1Message>().await;
    let result = proxy
        .call_timeout(
            TestSleepAdd1Message(1000),
            std::time::Duration::from_millis(1500),
        )
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().unwrap(), 1001);
}

#[crate::test]
async fn test_single_proxy_single_message_timeout_delayed() {
    let actor = TestActor.spawn().await.unwrap();
    let proxy = actor.proxy::<TestActor, TestSleepAdd1Message>().await;
    let result = proxy
        .call_timeout(
            TestSleepAdd1Message(1500),
            std::time::Duration::from_millis(1000),
        )
        .await;
    assert!(result.unwrap().is_none());
}

#[crate::test]
async fn test_proxy_to_dead_actor() {
    let actor = TestActor.spawn().await.unwrap();
    let proxy = actor.proxy::<TestActor, TestAdd1Message>().await;
    actor.stop(Ok(()));
    let result = proxy.call(TestAdd1Message(1)).await;
    assert!(result.is_err());
}

#[crate::test]
async fn test_proxy_unblocked_to_dead_actor() {
    let actor = TestActor.spawn().await.unwrap();
    let proxy = actor.proxy::<TestActor, TestAdd1Message>().await;
    actor.stop(Ok(()));
    let result = proxy.call_unblock(TestAdd1Message(1)).await.await;
    assert!(result.is_err());
}
