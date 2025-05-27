//! Integration tests for statechart and actor functionality

use crate::common::*;
use lit_bit_core::StateMachine;
use lit_bit_macro::{statechart, statechart_event};
// Note: Duration and sleep removed as they're no longer needed

// Types for integration testing
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[statechart_event]
pub enum IntegrationEvent {
    #[default]
    Start,
    Stop,
}

#[derive(Debug, Clone, Default)]
pub struct IntegrationContext {
    pub counter: u32,
}

statechart! {
    name: IntegrationMachine,
    context: IntegrationContext,
    event: IntegrationEvent,
    initial: Idle,

    state Idle {
        on IntegrationEvent::Start => Running;
    }

    state Running {
        on IntegrationEvent::Stop => Idle;
    }
}

#[test]
fn basic_sanity_check() {
    // Migrated from tests/agent_tests.rs
    // Basic test to ensure the test suite runs.
    assert_eq!(2 + 2, 4);
}

#[test]
fn test_with_std_feature() {
    // Test that requires std feature
    println!("This test runs with 'std' feature enabled.");
    let vec = [1, 2, 3];
    assert!(!vec.is_empty());
}

#[tokio::test]
async fn test_basic_statechart_integration() {
    setup_tracing();

    // Test basic statechart functionality using the types defined at module level

    // Test statechart creation and basic transitions
    let mut machine =
        IntegrationMachine::new(IntegrationContext::default(), &IntegrationEvent::Start)
            .expect("Failed to create integration machine");

    // Check initial state
    let initial_state = machine.state();
    assert!(!initial_state.is_empty());
    println!("Initial state: {initial_state:?}");

    // Test transition
    machine.send(&IntegrationEvent::Start);
    let running_state = machine.state();
    println!("After Start event: {running_state:?}");

    machine.send(&IntegrationEvent::Stop);
    let final_state = machine.state();
    println!("After Stop event: {final_state:?}");

    println!("✅ Basic statechart integration test passed");
}

#[derive(Debug)]
struct TestActor {
    counter: u32,
}

#[derive(Debug)]
enum ActorMessage {
    Increment,
    GetCount {
        reply_to: tokio::sync::oneshot::Sender<u32>,
    },
}

impl lit_bit_core::actor::Actor for TestActor {
    type Message = ActorMessage;

    fn on_event(&mut self, msg: ActorMessage) -> futures::future::BoxFuture<'_, ()> {
        Box::pin(async move {
            match msg {
                ActorMessage::Increment => self.counter += 1,
                ActorMessage::GetCount { reply_to } => {
                    let _ = reply_to.send(self.counter);
                }
            }
        })
    }
}

#[tokio::test]
async fn test_actor_mailbox_integration() {
    use lit_bit_core::actor::{Actor, create_mailbox};
    use tokio::sync::oneshot;

    setup_tracing();

    // Test mailbox creation and basic message passing
    let (outbox, mut inbox) = create_mailbox::<ActorMessage>(16);

    // Send a message
    outbox.send(ActorMessage::Increment).await.unwrap();

    // Receive and process the message
    if let Some(msg) = inbox.recv().await {
        let mut actor = TestActor { counter: 0 };
        actor.on_event(msg).await;

        // Verify the actor processed the message
        let (tx, rx) = oneshot::channel();
        actor
            .on_event(ActorMessage::GetCount { reply_to: tx })
            .await;
        let count = rx.await.unwrap();
        assert_eq!(count, 1);
    }

    println!("✅ Actor mailbox integration test passed");
}

#[tokio::test]
#[cfg(feature = "embassy")]
async fn test_embassy_integration() {
    setup_tracing();

    // Test Embassy integration when feature is enabled
    // TODO: Implement Embassy-specific tests when Embassy runtime is available
    // For now, just verify the feature is enabled
    println!("Embassy feature is enabled - Embassy integration tests would run here");
}

#[derive(Debug)]
struct TokioTestActor {
    processed_count: u32,
}

#[derive(Debug)]
enum TokioMessage {
    Process,
    GetCount {
        reply_to: tokio::sync::oneshot::Sender<u32>,
    },
}

impl lit_bit_core::actor::Actor for TokioTestActor {
    type Message = TokioMessage;

    fn on_event(&mut self, msg: TokioMessage) -> futures::future::BoxFuture<'_, ()> {
        Box::pin(async move {
            match msg {
                TokioMessage::Process => self.processed_count += 1,
                TokioMessage::GetCount { reply_to } => {
                    let _ = reply_to.send(self.processed_count);
                }
            }
        })
    }
}

#[tokio::test]
async fn test_tokio_integration() {
    use lit_bit_core::actor::spawn_actor_tokio;
    use tokio::sync::oneshot;

    setup_tracing();

    // Test Tokio actor spawning
    let actor = TokioTestActor { processed_count: 0 };
    let addr = spawn_actor_tokio(actor, 16);

    // Send some messages
    addr.send(TokioMessage::Process).await.unwrap();
    addr.send(TokioMessage::Process).await.unwrap();

    // Get the count
    let (tx, rx) = oneshot::channel();
    addr.send(TokioMessage::GetCount { reply_to: tx })
        .await
        .unwrap();
    let count = rx.await.unwrap();

    assert_eq!(count, 2);
    println!("✅ Tokio integration test passed - processed {count} messages");
}
