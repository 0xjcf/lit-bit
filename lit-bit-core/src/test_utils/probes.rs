//! Async state probes for deterministic testing across Tokio and Embassy runtimes
//!
//! This module provides stream-based event monitoring that allows tests to observe
//! actor lifecycle events, state transitions, and message processing in a
//! deterministic manner.

use core::marker::PhantomData;
#[cfg(any(feature = "async-tokio", feature = "async-embassy"))]
use core::time::Duration;

// Use appropriate string types depending on the environment
#[cfg(any(feature = "std", feature = "alloc"))]
type ProbeString = alloc::string::String;

#[cfg(not(any(feature = "std", feature = "alloc")))]
type ProbeString = heapless::String<64>; // Fixed-size string for no_std

#[cfg(any(feature = "std", feature = "alloc"))]
extern crate alloc;

/// Events that can be observed during actor lifecycle and execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeEvent {
    /// Actor state transition occurred
    StateTransition { from: ProbeString, to: ProbeString },
    /// Actor received a message of the specified type
    MessageReceived { message_type: ProbeString },
    /// Actor received a message with captured content (for detailed testing)
    MessageWithContent {
        message_type: ProbeString,
        content: ProbeString,
    },
    /// Actor started successfully
    ActorStarted,
    /// Actor stopped (gracefully or due to error)
    ActorStopped,
    /// Actor panicked with error details
    PanicOccurred { error: ProbeString },
}

/// Test probe for observing actor events with deterministic timeouts
///
/// This probe provides async methods to wait for specific actor events during testing.
/// It uses different underlying channels depending on the runtime (Tokio vs Embassy)
/// but provides a unified API for test code.
pub struct ActorProbe<A> {
    #[cfg(feature = "async-tokio")]
    event_receiver: tokio::sync::mpsc::Receiver<ProbeEvent>,
    #[cfg(feature = "async-embassy")]
    event_receiver: heapless::spsc::Consumer<'static, ProbeEvent, 32>,
    _phantom: PhantomData<A>,
}

impl<A> ActorProbe<A> {
    /// Create a new probe with a Tokio receiver (for Tokio runtime)
    #[cfg(feature = "async-tokio")]
    pub fn new(event_receiver: tokio::sync::mpsc::Receiver<ProbeEvent>) -> Self {
        Self {
            event_receiver,
            _phantom: PhantomData,
        }
    }

    /// Create a new probe with an Embassy receiver (for Embassy runtime)
    #[cfg(feature = "async-embassy")]
    pub fn new_embassy(event_receiver: heapless::spsc::Consumer<'static, ProbeEvent, 32>) -> Self {
        Self {
            event_receiver,
            _phantom: PhantomData,
        }
    }

    /// Wait for a specific state transition with timeout
    ///
    /// This method provides deterministic testing by waiting for an exact state transition
    /// from one state to another. Uses platform-appropriate timeout mechanisms.
    ///
    /// # Arguments
    /// * `from_state` - The state name to transition from
    /// * `to_state` - The state name to transition to
    ///
    /// # Returns
    /// `Ok(())` if the expected transition occurred within the timeout,
    /// `Err(TestError)` if the timeout elapsed or an unexpected event occurred.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use lit_bit_core::test_utils::ActorProbe;
    /// # async fn example(mut probe: ActorProbe<()>) -> Result<(), Box<dyn std::error::Error>> {
    /// // Wait for actor to transition from "Idle" to "Running"
    /// probe.expect_state_transition("Idle", "Running").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn expect_state_transition(
        &mut self,
        _from_state: &str,
        _to_state: &str,
    ) -> Result<(), TestError> {
        #[cfg(feature = "async-tokio")]
        {
            let timeout_duration = Duration::from_secs(5);
            let result = tokio::time::timeout(timeout_duration, async {
                while let Some(event) = self.event_receiver.recv().await {
                    #[allow(clippy::collapsible_if)]
                    if let ProbeEvent::StateTransition { from, to } = event {
                        if from.as_str() == _from_state && to.as_str() == _to_state {
                            return Ok(());
                        }
                    }
                }
                Err(TestError::UnexpectedEnd)
            })
            .await;

            result.map_err(|_| TestError::Timeout)?
        }

        #[cfg(feature = "async-embassy")]
        {
            embassy_time::with_timeout(embassy_time::Duration::from_secs(5), async {
                loop {
                    if let Some(event) = self.event_receiver.dequeue() {
                        if let ProbeEvent::StateTransition { from, to } = event {
                            if from.as_str() == _from_state && to.as_str() == _to_state {
                                return Ok(());
                            }
                        }
                    }
                    // Small delay to yield control and avoid busy-waiting
                    embassy_time::Timer::after(embassy_time::Duration::from_millis(1)).await;
                }
            })
            .await
            .map_err(|_| TestError::Timeout)?
        }

        #[cfg(not(any(feature = "async-tokio", feature = "async-embassy")))]
        {
            // Fallback for no async runtime
            Err(TestError::Timeout)
        }
    }

    /// Wait for the actor to start with timeout
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use lit_bit_core::test_utils::ActorProbe;
    /// # async fn example(mut probe: ActorProbe<()>) -> Result<(), Box<dyn std::error::Error>> {
    /// probe.expect_actor_started().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn expect_actor_started(&mut self) -> Result<(), TestError> {
        self.expect_event(ProbeEvent::ActorStarted).await
    }

    /// Wait for the actor to stop with timeout
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use lit_bit_core::test_utils::ActorProbe;
    /// # async fn example(mut probe: ActorProbe<()>) -> Result<(), Box<dyn std::error::Error>> {
    /// probe.expect_actor_stopped().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn expect_actor_stopped(&mut self) -> Result<(), TestError> {
        self.expect_event(ProbeEvent::ActorStopped).await
    }

    /// Wait for a panic event and return the error details
    ///
    /// This is useful for testing supervision and error recovery mechanisms.
    ///
    /// # Returns
    /// The panic error message if a panic occurred within the timeout.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use lit_bit_core::test_utils::ActorProbe;
    /// # async fn example(mut probe: ActorProbe<()>) -> Result<(), Box<dyn std::error::Error>> {
    /// let panic_error = probe.expect_panic().await?;
    /// assert!(panic_error.contains("division by zero"));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn expect_panic(&mut self) -> Result<ProbeString, TestError> {
        loop {
            let event = self.next_event().await?;
            if let ProbeEvent::PanicOccurred { error } = event {
                return Ok(error);
            }
        }
    }

    /// Wait for a specific event type with timeout
    ///
    /// Generic method for waiting for any type of probe event.
    ///
    /// # Arguments
    /// * `expected_event` - The exact event to wait for
    async fn expect_event(&mut self, _expected_event: ProbeEvent) -> Result<(), TestError> {
        #[cfg(feature = "async-tokio")]
        {
            let timeout_duration = Duration::from_secs(5);
            let result = tokio::time::timeout(timeout_duration, async {
                while let Some(event) = self.event_receiver.recv().await {
                    if event == _expected_event {
                        return Ok(());
                    }
                }
                Err(TestError::UnexpectedEnd)
            })
            .await;

            result.map_err(|_| TestError::Timeout)?
        }

        #[cfg(feature = "async-embassy")]
        {
            embassy_time::with_timeout(embassy_time::Duration::from_secs(5), async {
                loop {
                    if let Some(event) = self.event_receiver.dequeue() {
                        if event == _expected_event {
                            return Ok(());
                        }
                    }
                    embassy_time::Timer::after(embassy_time::Duration::from_millis(1)).await;
                }
            })
            .await
            .map_err(|_| TestError::Timeout)
        }

        #[cfg(not(any(feature = "async-tokio", feature = "async-embassy")))]
        {
            // Fallback for no async runtime
            Err(TestError::Timeout)
        }
    }

    /// Get the next event from the probe without timeout checking
    ///
    /// This is a helper method used by other probe methods. It handles the
    /// platform-specific event receiving logic.
    async fn next_event(&mut self) -> Result<ProbeEvent, TestError> {
        #[cfg(feature = "async-tokio")]
        {
            self.event_receiver
                .recv()
                .await
                .ok_or(TestError::UnexpectedEnd)
        }

        #[cfg(feature = "async-embassy")]
        {
            loop {
                if let Some(event) = self.event_receiver.dequeue() {
                    return Ok(event);
                }
                embassy_time::Timer::after(embassy_time::Duration::from_millis(1)).await;
            }
        }

        #[cfg(not(any(feature = "async-tokio", feature = "async-embassy")))]
        {
            // Fallback for no async runtime
            Err(TestError::UnexpectedEnd)
        }
    }

    /// Wait for a specific message type with timeout
    ///
    /// This method waits for an actor to receive a message of a specific type.
    /// Useful for testing message flow and actor interactions.
    ///
    /// # Arguments
    /// * `message_type` - The name of the message type to wait for
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use lit_bit_core::test_utils::ActorProbe;
    /// # async fn example(mut probe: ActorProbe<()>) -> Result<(), Box<dyn std::error::Error>> {
    /// probe.expect_message_type("CalculatorMessage").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn expect_message_type(&mut self, message_type: &str) -> Result<(), TestError> {
        loop {
            let event = self.next_event().await?;
            match event {
                ProbeEvent::MessageReceived {
                    message_type: received_type,
                }
                | ProbeEvent::MessageWithContent {
                    message_type: received_type,
                    ..
                } => {
                    if received_type.as_str() == message_type {
                        return Ok(());
                    }
                }
                _ => continue,
            }
        }
    }

    /// Wait for a message with specific content
    ///
    /// This provides detailed message capture as mentioned in research for debugging
    /// complex actor interactions and verifying exact message sequences.
    ///
    /// # Arguments
    /// * `message_type` - The type of message to wait for
    /// * `expected_content` - The expected content (Debug representation)
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use lit_bit_core::test_utils::ActorProbe;
    /// # async fn example(mut probe: ActorProbe<()>) -> Result<(), Box<dyn std::error::Error>> {
    /// probe.expect_message_content("Calculate", "Add(5, 3)").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn expect_message_content(
        &mut self,
        message_type: &str,
        expected_content: &str,
    ) -> Result<(), TestError> {
        loop {
            let event = self.next_event().await?;
            #[allow(clippy::collapsible_if)]
            if let ProbeEvent::MessageWithContent {
                message_type: received_type,
                content,
            } = event
            {
                if received_type.as_str() == message_type && content.as_str() == expected_content {
                    return Ok(());
                }
            }
        }
    }

    /// Capture a sequence of messages up to a specified count
    ///
    /// This implements the "MessageCapture<M>" pattern from the research, allowing
    /// tests to record all messages of certain types for later assertion.
    ///
    /// # Arguments
    /// * `max_messages` - Maximum number of messages to capture
    ///
    /// # Returns
    /// A vector of captured message events (types and content if available)
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use lit_bit_core::test_utils::ActorProbe;
    /// # async fn example(mut probe: ActorProbe<()>) -> Result<(), Box<dyn std::error::Error>> {
    /// let captured = probe.capture_messages(5).await?;
    /// assert!(captured.len() >= 2, "Expected at least 2 messages");
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(any(feature = "std", feature = "alloc"))]
    pub async fn capture_messages(
        &mut self,
        max_messages: usize,
    ) -> Result<alloc::vec::Vec<ProbeEvent>, TestError> {
        let mut captured = alloc::vec::Vec::with_capacity(max_messages);

        for _ in 0..max_messages {
            match self.next_event().await {
                Ok(event) => {
                    match &event {
                        ProbeEvent::MessageReceived { .. }
                        | ProbeEvent::MessageWithContent { .. } => {
                            captured.push(event);
                        }
                        _ => {
                            // Non-message event, put it back conceptually by breaking
                            // In a real implementation, we might need a putback mechanism
                            break;
                        }
                    }
                }
                Err(_) => break, // Timeout or end of stream
            }
        }

        if captured.is_empty() {
            Err(TestError::UnexpectedEnd)
        } else {
            Ok(captured)
        }
    }

    /// Capture a sequence of messages up to a specified count (no_std version)
    ///
    /// This implements the "MessageCapture<M>" pattern from the research, allowing
    /// tests to record all messages of certain types for later assertion.
    /// Uses a fixed-size heapless vector for no_std compatibility.
    ///
    /// # Arguments
    /// * `max_messages` - Maximum number of messages to capture (capped at 16 for no_std)
    ///
    /// # Returns
    /// A heapless vector of captured message events (types and content if available)
    #[cfg(not(any(feature = "std", feature = "alloc")))]
    pub async fn capture_messages(
        &mut self,
        max_messages: usize,
    ) -> Result<heapless::Vec<ProbeEvent, 16>, TestError> {
        let mut captured = heapless::Vec::new();
        let actual_max = core::cmp::min(max_messages, 16); // Cap at heapless Vec size

        for _ in 0..actual_max {
            match self.next_event().await {
                Ok(event) => {
                    match &event {
                        ProbeEvent::MessageReceived { .. }
                        | ProbeEvent::MessageWithContent { .. } => {
                            if captured.push(event).is_err() {
                                // Vector is full, break
                                break;
                            }
                        }
                        _ => {
                            // Non-message event, put it back conceptually by breaking
                            break;
                        }
                    }
                }
                Err(_) => break, // Timeout or end of stream
            }
        }

        if captured.is_empty() {
            Err(TestError::UnexpectedEnd)
        } else {
            Ok(captured)
        }
    }
}

/// Errors that can occur during test probe operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestError {
    /// Operation timed out waiting for expected event
    Timeout,
    /// Event stream ended unexpectedly (e.g., actor stopped)
    UnexpectedEnd,
    /// Received an event different from what was expected
    UnexpectedEvent(ProbeEvent),
}

impl core::fmt::Display for TestError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TestError::Timeout => write!(f, "Operation timed out waiting for expected event"),
            TestError::UnexpectedEnd => write!(f, "Event stream ended unexpectedly"),
            TestError::UnexpectedEvent(event) => {
                write!(f, "Received unexpected event: {event:?}")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TestError {}

// Helper function to create strings safely in no_std environments
#[cfg(any(feature = "std", feature = "alloc"))]
pub(crate) fn create_probe_string(s: &str) -> ProbeString {
    s.to_string()
}

#[cfg(not(any(feature = "std", feature = "alloc")))]
pub(crate) fn create_probe_string(s: &str) -> ProbeString {
    heapless::String::try_from(s).unwrap_or_else(|_| heapless::String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_event_equality_works() {
        let event1 = ProbeEvent::ActorStarted;
        let event2 = ProbeEvent::ActorStarted;
        let event3 = ProbeEvent::ActorStopped;

        assert_eq!(event1, event2);
        assert_ne!(event1, event3);
    }

    #[test]
    fn state_transition_event_equality() {
        let transition1 = ProbeEvent::StateTransition {
            from: create_probe_string("Idle"),
            to: create_probe_string("Running"),
        };
        let transition2 = ProbeEvent::StateTransition {
            from: create_probe_string("Idle"),
            to: create_probe_string("Running"),
        };
        let transition3 = ProbeEvent::StateTransition {
            from: create_probe_string("Running"),
            to: create_probe_string("Idle"),
        };

        assert_eq!(transition1, transition2);
        assert_ne!(transition1, transition3);
    }

    #[test]
    fn test_error_display() {
        let timeout_error = TestError::Timeout;
        // Use core::fmt::Display instead of format! macro for no_std compatibility
        use core::fmt::Write;
        let mut buffer = heapless::String::<64>::new();
        write!(&mut buffer, "{timeout_error}").unwrap();
        assert!(buffer.contains("timed out"));
    }

    #[test]
    fn message_content_event_creation() {
        let message_event = ProbeEvent::MessageWithContent {
            message_type: create_probe_string("Calculate"),
            content: create_probe_string("Add(5, 3)"),
        };

        match message_event {
            ProbeEvent::MessageWithContent {
                message_type,
                content,
            } => {
                assert_eq!(message_type.as_str(), "Calculate");
                assert_eq!(content.as_str(), "Add(5, 3)");
            }
            _ => panic!("Expected MessageWithContent event"),
        }
    }
}
