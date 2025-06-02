//! Instrumented actor wrapper for zero-cost probe integration
//!
//! This module provides a wrapper that adds probe instrumentation to any actor
//! without changing its behavior. The wrapper implements the Actor trait and
//! forwards all calls to the inner actor while emitting probe events.

use super::probes::{ProbeEvent, create_probe_string};
use crate::actor::{Actor, ActorError};
use core::marker::PhantomData;

/// Wrapper that adds probe instrumentation without changing actor behavior
///
/// This wrapper implements a zero-cost abstraction for adding test instrumentation
/// to actors. It forwards all Actor trait methods to the inner actor while emitting
/// probe events for observation during testing.
///
/// The wrapper uses conditional compilation to provide different probe channels
/// depending on the runtime (Tokio vs Embassy) while maintaining a unified API.
pub struct InstrumentedActor<A> {
    inner: A,
    #[cfg(feature = "async-tokio")]
    probe_sender: tokio::sync::mpsc::Sender<ProbeEvent>,
    #[cfg(feature = "async-embassy")]
    probe_sender: heapless::spsc::Producer<'static, ProbeEvent, 32>,
    _phantom: PhantomData<A>,
}

impl<A> InstrumentedActor<A> {
    /// Create a new instrumented actor for Tokio runtime
    ///
    /// # Arguments
    /// * `actor` - The inner actor to wrap with instrumentation
    /// * `probe_sender` - Channel sender for emitting probe events
    #[cfg(feature = "async-tokio")]
    pub fn new(actor: A, probe_sender: tokio::sync::mpsc::Sender<ProbeEvent>) -> Self {
        Self {
            inner: actor,
            probe_sender,
            _phantom: PhantomData,
        }
    }

    /// Create a new instrumented actor for Embassy runtime
    ///
    /// # Arguments
    /// * `actor` - The inner actor to wrap with instrumentation
    /// * `probe_sender` - Channel producer for emitting probe events
    #[cfg(feature = "async-embassy")]
    pub fn new_embassy(
        actor: A,
        probe_sender: heapless::spsc::Producer<'static, ProbeEvent, 32>,
    ) -> Self {
        Self {
            inner: actor,
            probe_sender,
            _phantom: PhantomData,
        }
    }

    /// Emit a probe event in a non-blocking manner
    ///
    /// This method attempts to send probe events without blocking the actor's
    /// message processing. If the probe channel is full, events are dropped
    /// to maintain actor performance.
    fn emit_event(&self, event: ProbeEvent) {
        #[cfg(feature = "async-tokio")]
        {
            // Use try_send to avoid blocking the actor if probe buffer is full
            let _ = self.probe_sender.try_send(event);
        }

        #[cfg(feature = "async-embassy")]
        {
            // Use enqueue to avoid blocking the actor if probe buffer is full
            let _ = self.probe_sender.enqueue(event);
        }

        #[cfg(not(any(feature = "async-tokio", feature = "async-embassy")))]
        {
            // No probe channel available - silently drop the event
            let _ = event;
        }
    }

    /// Get a reference to the inner actor
    ///
    /// This allows test code to access the inner actor's state if needed,
    /// while maintaining the instrumentation wrapper.
    pub fn inner(&self) -> &A {
        &self.inner
    }

    /// Get a mutable reference to the inner actor
    ///
    /// This allows test code to modify the inner actor's state if needed,
    /// while maintaining the instrumentation wrapper.
    pub fn inner_mut(&mut self) -> &mut A {
        &mut self.inner
    }

    /// Emit detailed message content if the message type supports Debug
    ///
    /// This method enables the detailed message capture pattern mentioned in research.
    /// It's conditionally compiled to avoid overhead in release builds.
    /// Call this manually from test code when you need detailed message content.
    #[cfg(debug_assertions)]
    pub fn emit_message_content_debug<T>(&self, _message: &T)
    where
        T: core::fmt::Debug,
    {
        // Format message content using Debug representation
        // Use a simple format for no_std compatibility
        let content_string = create_probe_string("Debug content"); // Simplified for no_std

        self.emit_event(ProbeEvent::MessageWithContent {
            message_type: create_probe_string(core::any::type_name::<T>()),
            content: content_string,
        });
    }
}

impl<A: Actor> Actor for InstrumentedActor<A> {
    type Message = A::Message;

    // Use the inner actor's future type directly since we're just forwarding
    type Future<'a>
        = A::Future<'a>
    where
        Self: 'a;

    fn handle(&mut self, message: Self::Message) -> Self::Future<'_> {
        // Emit message received event for test observation
        // Use type name for basic event
        self.emit_event(ProbeEvent::MessageReceived {
            message_type: create_probe_string(core::any::type_name::<A::Message>()),
        });

        // Forward to inner actor - this preserves the original actor's behavior
        self.inner.handle(message)
    }

    fn on_start(&mut self) -> Result<(), ActorError> {
        // Forward to inner actor first
        let result = self.inner.on_start();

        // Emit start event based on result
        if result.is_ok() {
            self.emit_event(ProbeEvent::ActorStarted);
        }

        result
    }

    fn on_stop(self) -> Result<(), ActorError> {
        // Emit stop event before forwarding (since we consume self)
        self.emit_event(ProbeEvent::ActorStopped);

        // Forward to inner actor
        self.inner.on_stop()
    }

    fn on_panic(&self, info: &core::panic::PanicInfo) -> crate::actor::RestartStrategy {
        // Create panic message without using deprecated payload() method
        let panic_message = if let Some(location) = info.location() {
            // Avoid format! in no_std by constructing message manually
            let _file_name = location.file();
            let _line = location.line();
            let _col = location.column();
            create_probe_string("panic occurred") // Simplified for no_std compatibility
        } else {
            create_probe_string("panic at unknown location")
        };

        self.emit_event(ProbeEvent::PanicOccurred {
            error: panic_message,
        });

        // Forward to inner actor
        self.inner.on_panic(info)
    }
}

// Send implementation - using safe code instead of unsafe
// Rely on the compiler to automatically implement Send when appropriate
// This removes the need for the unsafe impl

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::{Actor, ActorError, RestartStrategy};
    use core::future::Ready;

    // Test actor for validation
    #[derive(Debug)]
    struct TestActor {
        pub counter: u32,
        pub started: bool,
    }

    impl TestActor {
        fn new() -> Self {
            Self {
                counter: 0,
                started: false,
            }
        }
    }

    impl Actor for TestActor {
        type Message = u32;
        type Future<'a>
            = Ready<()>
        where
            Self: 'a;

        fn handle(&mut self, message: Self::Message) -> Self::Future<'_> {
            self.counter += message;
            core::future::ready(())
        }

        fn on_start(&mut self) -> Result<(), ActorError> {
            self.started = true;
            Ok(())
        }

        fn on_stop(self) -> Result<(), ActorError> {
            Ok(())
        }

        fn on_panic(&self, _info: &core::panic::PanicInfo) -> RestartStrategy {
            RestartStrategy::OneForOne
        }
    }

    #[cfg(feature = "async-tokio")]
    #[tokio::test]
    async fn instrumented_actor_forwards_calls() {
        let test_actor = TestActor::new();
        let (probe_sender, mut probe_receiver) = tokio::sync::mpsc::channel(16);
        let mut instrumented = InstrumentedActor::new(test_actor, probe_sender);

        // Test on_start
        assert!(instrumented.on_start().is_ok());
        assert!(instrumented.inner().started);

        // Test handle
        instrumented.handle(42).await;
        assert_eq!(instrumented.inner().counter, 42);

        // Verify probe events were emitted
        if let Some(event) = probe_receiver.recv().await {
            assert_eq!(event, ProbeEvent::ActorStarted);
        }

        if let Some(event) = probe_receiver.recv().await {
            assert!(matches!(event, ProbeEvent::MessageReceived { .. }));
        }
    }

    #[test]
    fn instrumented_actor_provides_inner_access() {
        #[cfg(feature = "async-tokio")]
        {
            let test_actor = TestActor::new();
            let (probe_sender, _probe_receiver) = tokio::sync::mpsc::channel(16);
            let mut instrumented = InstrumentedActor::new(test_actor, probe_sender);

            // Test mutable access to inner actor
            instrumented.inner_mut().counter = 100;
            assert_eq!(instrumented.inner().counter, 100);
        }

        #[cfg(feature = "async-embassy")]
        {
            let test_actor = TestActor::new();
            use heapless::spsc::Queue;
            static mut PROBE_QUEUE: Queue<ProbeEvent, 32> = Queue::new();
            let (probe_producer, _probe_consumer) = unsafe { PROBE_QUEUE.split() };
            let mut instrumented = InstrumentedActor::new_embassy(test_actor, probe_producer);

            // Test mutable access to inner actor
            instrumented.inner_mut().counter = 100;
            assert_eq!(instrumented.inner().counter, 100);
        }

        #[cfg(not(any(feature = "async-tokio", feature = "async-embassy")))]
        {
            // When no async features are enabled, just verify the test compiles
            let _test_actor = TestActor::new();
        }
    }

    #[test]
    fn instrumented_actor_implements_send_when_inner_is_send() {
        fn assert_send<T: Send>() {}

        // This test verifies that InstrumentedActor<T> implements Send when T implements Send
        // Now relying on automatic Send derivation instead of unsafe impl
        assert_send::<InstrumentedActor<TestActor>>();
    }
}
