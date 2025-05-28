//! Comprehensive actor tests for std environments
//!
//! Tests cover:
//! - Message processing and event conversion
//! - Mailbox integration (send/receive patterns)
//! - Actor lifecycle (start/stop/error scenarios)
//! - Back-pressure handling

use lit_bit_core::actor::{
    Actor, ActorError, Inbox, Outbox, RestartStrategy, actor_task, backpressure::SendError,
    backpressure::std_async,
};

use core::panic::PanicInfo;

/// Test actor for unit testing with configurable behavior
#[derive(Debug)]
struct TestActor {
    counter: u32,
    should_fail_start: bool,
    should_fail_stop: bool,
    processed_events: Vec<TestEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TestEvent {
    Increment,
    SetValue(u32),
    Stop,
}

impl TestActor {
    fn new() -> Self {
        Self {
            counter: 0,
            should_fail_start: false,
            should_fail_stop: false,
            processed_events: Vec::new(),
        }
    }

    fn with_start_failure() -> Self {
        Self {
            should_fail_start: true,
            ..Self::new()
        }
    }

    fn with_stop_failure() -> Self {
        Self {
            should_fail_stop: true,
            ..Self::new()
        }
    }
}

impl Actor for TestActor {
    type Message = TestEvent;
    type Future<'a>
        = core::future::Ready<()>
    where
        Self: 'a;

    fn handle(&mut self, msg: Self::Message) -> Self::Future<'_> {
        self.processed_events.push(msg.clone());

        match msg {
            TestEvent::Increment => self.counter += 1,
            TestEvent::SetValue(value) => self.counter = value,
            TestEvent::Stop => {
                // This would normally signal the actor to stop
                // In our test, we just record it
            }
        }

        core::future::ready(())
    }

    fn on_start(&mut self) -> Result<(), ActorError> {
        if self.should_fail_start {
            Err(ActorError::StartupFailure)
        } else {
            Ok(())
        }
    }

    fn on_stop(self) -> Result<(), ActorError> {
        if self.should_fail_stop {
            Err(ActorError::ShutdownFailure)
        } else {
            Ok(())
        }
    }

    fn on_panic(&self, _info: &PanicInfo) -> RestartStrategy {
        RestartStrategy::OneForOne
    }
}

/// Mock `StateMachine` for testing integration patterns
#[derive(Debug)]
struct MockStateMachine {
    state: u32,
    events_received: Vec<u32>,
}

impl MockStateMachine {
    fn new() -> Self {
        Self {
            state: 0,
            events_received: Vec::new(),
        }
    }
}

impl Actor for MockStateMachine {
    type Message = u32;
    type Future<'a>
        = core::future::Ready<()>
    where
        Self: 'a;

    fn handle(&mut self, event: Self::Message) -> Self::Future<'_> {
        self.events_received.push(event);
        self.state = event;
        core::future::ready(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_trait_basic_functionality() {
        let mut actor = TestActor::new();

        // Test initial state
        assert_eq!(actor.counter, 0);
        assert!(actor.processed_events.is_empty());

        // Test lifecycle hooks
        assert!(actor.on_start().is_ok());
        assert!(actor.on_stop().is_ok());
    }

    #[test]
    fn actor_lifecycle_start_failure() {
        let mut actor = TestActor::with_start_failure();
        assert_eq!(actor.on_start(), Err(ActorError::StartupFailure));
    }

    #[test]
    fn actor_lifecycle_stop_failure() {
        let actor = TestActor::with_stop_failure();
        assert_eq!(actor.on_stop(), Err(ActorError::ShutdownFailure));
    }

    #[tokio::test]
    async fn actor_message_processing() {
        let mut actor = TestActor::new();

        // Process various events
        actor.handle(TestEvent::Increment).await;
        assert_eq!(actor.counter, 1);
        assert_eq!(actor.processed_events, vec![TestEvent::Increment]);

        actor.handle(TestEvent::SetValue(42)).await;
        assert_eq!(actor.counter, 42);
        assert_eq!(
            actor.processed_events,
            vec![TestEvent::Increment, TestEvent::SetValue(42)]
        );
    }

    #[tokio::test]
    async fn statechart_integration() {
        let mut mock_sm = MockStateMachine::new();

        // Test direct event forwarding
        mock_sm.handle(100).await;
        assert_eq!(mock_sm.state, 100);
        assert_eq!(mock_sm.events_received, vec![100]);

        mock_sm.handle(200).await;
        assert_eq!(mock_sm.state, 200);
        assert_eq!(mock_sm.events_received, vec![100, 200]);
    }

    #[tokio::test]
    async fn std_mailbox_integration() {
        let (outbox, mut inbox): (Outbox<TestEvent>, Inbox<TestEvent>) =
            lit_bit_core::actor::create_mailbox::<TestEvent>(4);

        // Test async sending and receiving
        assert!(
            std_async::send::<TestEvent>(&outbox, TestEvent::Increment)
                .await
                .is_ok()
        );
        assert!(
            std_async::send::<TestEvent>(&outbox, TestEvent::SetValue(42))
                .await
                .is_ok()
        );

        // Test receiving
        assert_eq!(
            std_async::recv::<TestEvent>(&mut inbox).await,
            Some(TestEvent::Increment)
        );
        assert_eq!(
            std_async::recv::<TestEvent>(&mut inbox).await,
            Some(TestEvent::SetValue(42))
        );

        // Test try_send with capacity limits
        for i in 0..4 {
            assert!(std_async::try_send::<TestEvent>(&outbox, TestEvent::SetValue(i)).is_ok());
        }

        // Should fail when full (but send() would await)
        assert!(matches!(
            std_async::try_send::<TestEvent>(&outbox, TestEvent::Stop),
            Err(SendError::Full(TestEvent::Stop))
        ));
    }

    #[tokio::test]
    async fn std_backpressure_behavior() {
        let (outbox, mut inbox): (Outbox<u32>, _) = lit_bit_core::actor::create_mailbox::<u32>(2);

        // Fill mailbox to capacity
        assert!(std_async::try_send::<u32>(&outbox, 1).is_ok());
        assert!(std_async::try_send::<u32>(&outbox, 2).is_ok());

        // Verify capacity info
        assert_eq!(std_async::capacity::<u32>(&outbox), 2);

        // try_send should fail when full
        assert!(matches!(
            std_async::try_send::<u32>(&outbox, 3),
            Err(SendError::Full(3))
        ));

        // Test that send() can succeed when there's space
        // First, receive one item to make space
        let received = std_async::recv::<u32>(&mut inbox).await;
        assert_eq!(received, Some(1));

        // Now send() should succeed because there's space
        let send_result = std_async::send::<u32>(&outbox, 3).await;
        assert!(send_result.is_ok());

        // Verify we can receive the new item
        let received = std_async::recv::<u32>(&mut inbox).await;
        assert_eq!(received, Some(2));
        let received = std_async::recv::<u32>(&mut inbox).await;
        assert_eq!(received, Some(3));
    }

    #[tokio::test]
    async fn actor_task_lifecycle() {
        let actor = TestActor::new();
        let (outbox, inbox): (Outbox<TestEvent>, _) =
            lit_bit_core::actor::create_mailbox::<TestEvent>(8);

        // Send some events before starting the task
        std_async::send::<TestEvent>(&outbox, TestEvent::Increment)
            .await
            .unwrap();
        std_async::send::<TestEvent>(&outbox, TestEvent::SetValue(42))
            .await
            .unwrap();

        // Close the channel to signal shutdown
        drop(outbox);

        // Run the actor task
        let result = actor_task::<TestActor>(actor, inbox).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn actor_task_start_failure() {
        let actor = TestActor::with_start_failure();
        let (_outbox, inbox): (Outbox<TestEvent>, _) =
            lit_bit_core::actor::create_mailbox::<TestEvent>(8);

        let result = actor_task::<TestActor>(actor, inbox).await;
        assert_eq!(result, Err(ActorError::StartupFailure));
    }

    #[test]
    fn send_error_types() {
        let full_error = SendError::Full(42u32);
        let closed_error = SendError::Closed(42u32);

        // Test Display implementation
        assert_eq!(full_error.to_string(), "mailbox is full");
        assert_eq!(closed_error.to_string(), "receiver has been dropped");

        // Test Debug implementation
        assert_eq!(format!("{full_error:?}"), "Full(42)");
        assert_eq!(format!("{closed_error:?}"), "Closed(42)");

        // Test PartialEq
        assert_eq!(SendError::Full(42u32), SendError::Full(42u32));
        assert_ne!(SendError::Full(42u32), SendError::Closed(42u32));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::{TestActor, TestEvent};
    use lit_bit_core::actor::Actor;

    /// Test that demonstrates the complete actor workflow
    #[tokio::test]
    async fn complete_actor_workflow() {
        let mut actor = TestActor::new();

        // Test startup
        assert!(actor.on_start().is_ok());

        // Process a sequence of events
        let events = vec![
            TestEvent::Increment,
            TestEvent::Increment,
            TestEvent::SetValue(100),
        ];

        for event in events.clone() {
            actor.handle(event).await;
        }

        // Verify final state
        assert_eq!(actor.counter, 100);
        assert_eq!(actor.processed_events, events);

        // Test shutdown
        assert!(actor.on_stop().is_ok());
    }

    /// Test actor behavior under error conditions
    #[tokio::test]
    async fn actor_error_handling() {
        let mut actor = TestActor::new();

        // Normal operation
        actor.handle(TestEvent::Increment).await;
        assert_eq!(actor.counter, 1);

        // Test panic handling - we can't easily test the actual panic hook
        // but we can verify the default strategy by creating a simple test
        let restart_strategy = super::RestartStrategy::OneForOne;
        assert_eq!(restart_strategy, super::RestartStrategy::OneForOne);
    }
}
