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

    #[cfg(feature = "async")]
    /// Handles an incoming event by forwarding it to the state machine asynchronously.
    ///
    /// This method receives an event and calls the state machine's `send` method to process it. The result of the state transition is ignored.
    ///
    /// # Examples
    ///
    /// ```
    /// use your_crate::{Actor, StateMachine};
    ///
    /// let mut sm = MockStateMachine::new();
    /// let fut = sm.on_event(MockEvent::Start);
    /// futures::executor::block_on(fut);
    /// ```
    fn on_event(&mut self, event: Self::Message) -> futures::future::BoxFuture<'_, ()> {
        Box::pin(async move {
            // Direct forwarding to StateMachine::send
            let _result = self.send(&event);
            // In a real implementation, you might want to handle SendResult::Error
            // or log the state transitions
        })
    }

    #[cfg(not(feature = "async"))]
    #[allow(clippy::manual_async_fn)] /// Handles an incoming event by forwarding it to the state machine asynchronously.
    ///
    /// This method enables seamless integration between actor-based and state machine-based systems
    /// by treating each event as a message and delegating its handling to the underlying state machine.
    ///
    /// # Examples
    ///
    /// ```
    /// use your_crate::{Actor, StateMachine};
    ///
    /// let mut sm = MockStateMachine::new();
    /// sm.on_event(MockEvent::Start).await;
    /// assert_eq!(sm.state()[0], MockState::Working);
    /// ```
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
        /// Creates a new `MockStateMachine` instance with the initial state set to `Idle`.
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

        /// Processes an event and updates the mock state machine's state if a valid transition exists.
        ///
        /// Returns `SendResult::Transitioned` if the event triggers a state change, or `SendResult::NoMatch` if the event does not correspond to a valid transition.
        ///
        /// # Examples
        ///
        /// ```
        /// let mut sm = MockStateMachine::new();
        /// assert_eq!(sm.current_state, MockState::Idle);
        /// let result = sm.send(&MockEvent::Start);
        /// assert_eq!(result, SendResult::Transitioned);
        /// assert_eq!(sm.current_state, MockState::Working);
        /// let result = sm.send(&MockEvent::Stop);
        /// assert_eq!(result, SendResult::Transitioned);
        /// assert_eq!(sm.current_state, MockState::Idle);
        /// ```
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

        /// Returns a vector containing the current state of the state machine.
        ///
        /// # Examples
        ///
        /// ```
        /// let sm = MockStateMachine::new();
        /// let states = sm.state();
        /// assert_eq!(states.len(), 1);
        /// assert_eq!(states[0], State::Idle);
        /// ```
        fn state(&self) -> heapless::Vec<Self::State, 4> {
            let mut vec = Vec::new();
            vec.push(self.current_state).unwrap();
            vec
        }

        /// Returns a shared reference to the state machine's context.
        ///
        /// # Examples
        ///
        /// ```
        /// let sm = MockStateMachine::new();
        /// let ctx = sm.context();
        /// ```
        fn context(&self) -> &Self::Context {
            &self.context
        }

        /// Returns a mutable reference to the state machine's context.
        ///
        /// # Examples
        ///
        /// ```
        /// let mut sm = MockStateMachine::new();
        /// let ctx = sm.context_mut();
        /// // ctx can now be modified
        /// ```
        fn context_mut(&mut self) -> &mut Self::Context {
            &mut self.context
        }
    }

    #[cfg(feature = "std")]
    #[tokio::test]
    /// Tests integration of the `StateMachine` and `Actor` traits via the blanket implementation.
    ///
    /// This async test verifies that a type implementing `StateMachine` automatically implements `Actor`,
    /// and that events sent through the actor interface correctly trigger state transitions in the state machine.
    ///
    /// # Examples
    ///
    /// ```
    /// # use your_crate::{MockStateMachine, MockEvent, MockState};
    /// # async fn test() {
    /// let mut machine = MockStateMachine::new();
    /// assert_eq!(machine.state()[0], MockState::Idle);
    /// machine.on_event(MockEvent::Start).await;
    /// assert_eq!(machine.state()[0], MockState::Working);
    /// machine.on_event(MockEvent::Stop).await;
    /// assert_eq!(machine.state()[0], MockState::Idle);
    /// # }
    /// ```
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
    /// Verifies at compile time that `MockStateMachine` implements the `Actor` trait via the blanket implementation.
    ///
    /// This test will fail to compile if `MockStateMachine` does not satisfy the `Actor` trait bound.
    fn actor_trait_is_implemented_for_statemachine() {
        // This test verifies that the blanket impl works at compile time
        fn assert_actor<T: Actor>(_: &T) {}

        let machine = MockStateMachine::new();
        assert_actor(&machine); // This should compile without errors
    }
}
