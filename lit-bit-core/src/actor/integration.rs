//! `StateMachine` integration examples showing how to implement Actor for statechart types.

use super::Actor;
use crate::{SendResult, StateMachine};

/// Example showing how to implement Actor for any `StateMachine` type.
/// This demonstrates the direct integration pattern from Task 1.4.
///
/// Note: This is a blanket implementation that automatically makes any `StateMachine`
/// also implement the Actor trait, enabling seamless integration between the
/// statechart and actor systems.
impl<SM> Actor for SM
where
    SM: StateMachine + Send,
    SM::Event: Send + 'static,
{
    type Message = SM::Event;

    #[cfg(feature = "async")]
    fn on_event(&mut self, event: Self::Message) -> futures::future::BoxFuture<'_, ()> {
        Box::pin(async move {
            // Forward event to StateMachine and handle the result
            match self.send(&event) {
                SendResult::Transitioned => {
                    #[cfg(feature = "std")]
                    tracing::debug!("State transition completed successfully");
                }
                SendResult::NoMatch => {
                    #[cfg(feature = "std")]
                    tracing::debug!("No matching transition found for event");
                }
                SendResult::Error(error) => {
                    #[cfg(feature = "std")]
                    tracing::error!("State transition error: {:?}", error);
                    #[cfg(not(feature = "std"))]
                    {
                        // No logging available in no_std context
                        // Errors are still handled but not logged
                        let _ = error; // Suppress unused variable warning
                    }
                }
            }
        })
    }

    #[cfg(not(feature = "async"))]
    #[allow(clippy::manual_async_fn)] // Need Send bound for thread safety
    fn on_event(&mut self, event: Self::Message) -> impl core::future::Future<Output = ()> + Send {
        async move {
            // Forward event to StateMachine and handle the result
            match self.send(&event) {
                SendResult::Transitioned => {
                    #[cfg(feature = "std")]
                    tracing::debug!("State transition completed successfully");
                }
                SendResult::NoMatch => {
                    #[cfg(feature = "std")]
                    tracing::debug!("No matching transition found for event");
                }
                SendResult::Error(error) => {
                    #[cfg(feature = "std")]
                    tracing::error!("State transition error: {:?}", error);
                    #[cfg(not(feature = "std"))]
                    {
                        // No logging available in no_std context
                        // Errors are still handled but not logged
                        let _ = error; // Suppress unused variable warning
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SendResult;
    use heapless::Vec;

    // Simple mock state machine for testing the integration pattern
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum MockState {
        Idle,
        Working,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    enum MockEvent {
        Start,
        Stop,
    }

    struct MockStateMachine {
        current_state: MockState,
        context: (),
    }

    impl MockStateMachine {
        fn new() -> Self {
            Self {
                current_state: MockState::Idle,
                context: (),
            }
        }
    }

    impl StateMachine for MockStateMachine {
        type State = MockState;
        type Event = MockEvent;
        type Context = ();

        fn send(&mut self, event: &Self::Event) -> SendResult {
            // Simple state transitions for testing
            match (self.current_state, event) {
                (MockState::Idle, MockEvent::Start) => {
                    self.current_state = MockState::Working;
                    SendResult::Transitioned
                }
                (MockState::Working, MockEvent::Stop) => {
                    self.current_state = MockState::Idle;
                    SendResult::Transitioned
                }
                _ => SendResult::NoMatch, // Ignore invalid transitions
            }
        }

        fn state(&self) -> heapless::Vec<Self::State, 4> {
            let mut vec = Vec::new();
            vec.push(self.current_state).unwrap();
            vec
        }

        fn context(&self) -> &Self::Context {
            &self.context
        }

        fn context_mut(&mut self) -> &mut Self::Context {
            &mut self.context
        }
    }

    #[cfg(feature = "std")]
    #[tokio::test]
    async fn statechart_actor_integration() {
        // Create a mock statechart
        let mut machine = MockStateMachine::new();

        // Verify it implements Actor automatically via the blanket impl
        assert_eq!(machine.state()[0], MockState::Idle);

        // Test the actor interface - this demonstrates the key integration pattern
        machine.on_event(MockEvent::Start).await;
        assert_eq!(machine.state()[0], MockState::Working);

        machine.on_event(MockEvent::Stop).await;
        assert_eq!(machine.state()[0], MockState::Idle);
    }

    #[test]
    fn actor_trait_is_implemented_for_statemachine() {
        // This test verifies that the blanket impl works at compile time
        fn assert_actor<T: Actor>(_: &T) {}

        let machine = MockStateMachine::new();
        assert_actor(&machine); // This should compile without errors
    }

    // Mock state machine that can return errors for testing error handling
    struct ErrorStateMachine {
        should_error: bool,
        context: (),
    }

    impl ErrorStateMachine {
        fn new(should_error: bool) -> Self {
            Self {
                should_error,
                context: (),
            }
        }
    }

    impl StateMachine for ErrorStateMachine {
        type State = MockState;
        type Event = MockEvent;
        type Context = ();

        fn send(&mut self, _event: &Self::Event) -> SendResult {
            if self.should_error {
                SendResult::Error(crate::runtime::ProcessingError::EntryLogicFailure)
            } else {
                SendResult::Transitioned
            }
        }

        fn state(&self) -> heapless::Vec<Self::State, 4> {
            let mut vec = Vec::new();
            vec.push(MockState::Idle).unwrap();
            vec
        }

        fn context(&self) -> &Self::Context {
            &self.context
        }

        fn context_mut(&mut self) -> &mut Self::Context {
            &mut self.context
        }
    }

    #[cfg(feature = "std")]
    #[tokio::test]
    async fn error_handling_integration() {
        // Test that error handling works without panicking
        let mut error_machine = ErrorStateMachine::new(true);

        // This should handle the error gracefully and not panic
        error_machine.on_event(MockEvent::Start).await;

        // Verify the machine is still functional
        assert_eq!(error_machine.state()[0], MockState::Idle);
    }
}
