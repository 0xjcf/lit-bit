//! `StateMachine` integration examples showing how to implement Actor for statechart types.

use super::Actor;
use crate::StateMachine;

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

    #[allow(clippy::manual_async_fn)] // Need Send bound for thread safety
    fn on_event(&mut self, event: Self::Message) -> impl core::future::Future<Output = ()> + Send {
        async move {
            // Direct forwarding to StateMachine::send
            let _result = self.send(&event);
            // In a real implementation, you might want to handle SendResult::Error
            // or log the state transitions
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
}
