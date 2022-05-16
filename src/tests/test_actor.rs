use super::*;

#[crate::test]
async fn test_strong_to_weak_to_strong() {
    let actor = TestActor.spawn().await.unwrap();
    let weak = actor.downgrade();
    let strong = weak.upgrade().unwrap();
    assert_eq!(
        strong
            .call::<TestActor, TestAdd1Message>(TestAdd1Message(1))
            .await
            .unwrap(),
        2
    );
}
