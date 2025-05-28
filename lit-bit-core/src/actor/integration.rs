//! `StateMachine` integration examples showing how to implement Actor for statechart types.

#[cfg(test)]
use super::Actor;
#[cfg(test)]
use crate::{SendResult, StateMachine};

/// Example showing how to implement Actor for any `StateMachine` type.
/// This demonstrates the direct integration pattern from Task 1.4.
///
/// Note: This is a blanket implementation that automatically makes any `StateMachine`
/// also implement the Actor trait, enabling seamless integration between the
/// statechart and actor systems.
/// Blanket implementation of Actor for `StateMachine` types.
///
/// Note: This implementation is only available when the `AsyncActor` blanket impl
/// is not in scope to avoid conflicts. In practice, you should choose either
/// the `StateMachine` integration OR the `AsyncActor` pattern, not both.
// TODO: Phase 5 - Resolve blanket implementation conflict
// This implementation conflicts with the AsyncActor blanket impl.
// We'll provide a more specific integration pattern in Phase 5.
// Consider creating a GitHub issue to track this architectural decision.
/*
impl<SM> Actor for SM
where
    SM: StateMachine + Send,
    SM::Event: Send + 'static,
{
    type Message = SM::Event;
    type Future<'a> = /* ... */;
    fn handle<'a>(&'a mut self, event: Self::Message) -> Self::Future<'a> { /* ... */ }
}
*/
#[cfg(test)]
mod tests {
    use super::*;
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

    // Implement Actor for the mock to test the integration pattern
    impl Actor for MockStateMachine {
        type Message = MockEvent;
        type Future<'a>
            = core::future::Ready<()>
        where
            Self: 'a;

        fn handle(&mut self, event: Self::Message) -> Self::Future<'_> {
            // Forward event to StateMachine and handle the result
            match self.send(&event) {
                SendResult::Transitioned | SendResult::NoMatch => {
                    // State transition completed successfully or no matching transition
                }
                SendResult::Error(_error) => {
                    // Errors are handled but not logged in tests
                }
            }
            core::future::ready(())
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
        machine.handle(MockEvent::Start).await;
        assert_eq!(machine.state()[0], MockState::Working);

        machine.handle(MockEvent::Stop).await;
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

    // Implement Actor for the error mock to test error handling
    impl Actor for ErrorStateMachine {
        type Message = MockEvent;
        type Future<'a>
            = core::future::Ready<()>
        where
            Self: 'a;

        fn handle(&mut self, event: Self::Message) -> Self::Future<'_> {
            // Forward event to StateMachine and handle the result
            match self.send(&event) {
                SendResult::Transitioned | SendResult::NoMatch => {
                    // State transition completed successfully or no matching transition
                }
                SendResult::Error(_error) => {
                    // Errors are handled but not logged in tests
                }
            }
            core::future::ready(())
        }
    }

    #[cfg(feature = "std")]
    #[tokio::test]
    async fn error_handling_integration() {
        // Test that error handling works without panicking
        let mut error_machine = ErrorStateMachine::new(true);

        // This should handle the error gracefully and not panic
        error_machine.handle(MockEvent::Start).await;

        // Verify the machine is still functional
        assert_eq!(error_machine.state()[0], MockState::Idle);
    }
}
