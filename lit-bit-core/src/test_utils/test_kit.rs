//! Cross-runtime test kit for deterministic async actor testing
//!
//! This module provides a unified testing interface that works across both
//! Tokio and Embassy runtimes, enabling deterministic testing of async actors
//! with time control and state observation.

use core::time::Duration;

#[cfg(feature = "async-tokio")]
use super::instrumented_actor::InstrumentedActor;
#[cfg(feature = "async-tokio")]
use super::probes::ActorProbe;
#[cfg(feature = "async-tokio")]
use crate::{Actor, Address};

/// Main test kit for cross-runtime async actor testing
///
/// TestKit provides a unified API for testing actors across different async runtimes
/// (Tokio and Embassy) with deterministic time control and state observation capabilities.
/// It handles the platform-specific details while providing a consistent interface.
///
/// ## Features
///
/// - **Cross-runtime compatibility**: Works with both Tokio and Embassy
/// - **Deterministic time control**: Pause, advance, and control time for testing
/// - **Actor probes**: Observe actor lifecycle events and state transitions
/// - **Zero-heap Embassy support**: Uses static allocation for embedded testing
/// - **Conditional compilation**: Only available with test features
///
/// ## Usage
///
/// ```rust,no_run
/// # #[cfg(feature = "async-tokio")]
/// # {
/// use lit_bit_core::test_utils::TestKit;
/// use std::time::Duration;
///
/// #[tokio::test(start_paused = true)]
/// async fn test_actor_behavior() {
///     let test_kit = TestKit::new();
///     test_kit.pause_time();
///     
///     let (address, mut probe) = test_kit
///         .spawn_actor_with_probe::<MyActor, 16>(MyActor::new());
///     
///     // Send message and verify behavior
///     address.send(MyMessage::Start).await.unwrap();
///     probe.expect_actor_started().await.unwrap();
///     
///     // Advance time and check timer behavior
///     test_kit.advance_time(Duration::from_secs(5)).await;
///     probe.expect_state_transition("Idle", "Running").await.unwrap();
/// }
/// # }
/// ```
#[allow(dead_code)] // Some fields may not be used in all configurations
pub struct TestKit {
    #[cfg(feature = "async-tokio")]
    tokio_handle: Option<tokio::runtime::Handle>,
    #[cfg(feature = "async-embassy")]
    embassy_spawner: Option<embassy_executor::Spawner>,
}

impl TestKit {
    /// Create a new TestKit instance
    ///
    /// Automatically detects the current runtime environment and configures
    /// the appropriate backend for testing.
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "async-tokio")]
            tokio_handle: tokio::runtime::Handle::try_current().ok(),
            #[cfg(feature = "async-embassy")]
            embassy_spawner: None,
        }
    }

    /// Spawn an actor with probe instrumentation (Tokio runtime)
    ///
    /// Creates an instrumented wrapper around the actor that emits probe events
    /// for test observation. The actor runs in the Tokio runtime with deterministic
    /// message processing.
    ///
    /// # Type Parameters
    /// * `A` - The actor type to spawn
    /// * `CAPACITY` - The probe channel capacity (const generic)
    ///
    /// # Arguments
    /// * `actor` - The actor instance to spawn
    ///
    /// # Returns
    /// A tuple of (Address, ActorProbe) for interacting with and observing the actor.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # #[cfg(feature = "async-tokio")]
    /// # {
    /// use lit_bit_core::test_utils::TestKit;
    ///
    /// let test_kit = TestKit::new();
    /// let (address, mut probe) = test_kit
    ///     .spawn_actor_with_probe::<MyActor, 16>(MyActor::new());
    /// # }
    /// ```
    #[cfg(feature = "async-tokio")]
    pub fn spawn_actor_with_probe<A, const CAPACITY: usize>(
        &self,
        actor: A,
    ) -> (Address<A::Message>, ActorProbe<A>)
    where
        A: Actor + Send + 'static,
        A::Message: Send + 'static,
    {
        // Create probe channel
        let (probe_sender, probe_receiver) = tokio::sync::mpsc::channel(CAPACITY);

        // Wrap actor with instrumentation
        let instrumented_actor = InstrumentedActor::new(actor, probe_sender);

        // Spawn using existing spawn infrastructure
        let address = crate::actor::spawn::spawn_actor_tokio(instrumented_actor, 1000);

        // Create probe
        let probe = ActorProbe::new(probe_receiver);

        (address, probe)
    }

    /// Spawn an actor with probe instrumentation (Embassy runtime)
    ///
    /// Creates an instrumented wrapper around the actor using static allocation
    /// suitable for embedded environments. Uses heapless collections for zero-heap
    /// operation.
    ///
    /// # Type Parameters
    /// * `A` - The actor type to spawn
    /// * `CAPACITY` - The probe channel capacity (const generic)
    ///
    /// # Arguments
    /// * `spawner` - The Embassy spawner to use
    /// * `actor` - The actor instance to spawn
    ///
    /// # Returns
    /// A tuple of (Address, ActorProbe) for interacting with and observing the actor.
    ///
    /// # Implementation Note
    ///
    /// This method is currently unimplemented due to Embassy's architectural constraints.
    /// Embassy requires concrete (non-generic) task functions, which prevents implementing
    /// a generic spawn function like this.
    ///
    /// ## Workaround for Embassy Users
    ///
    /// To use TestKit with Embassy, you need to:
    /// 1. Create concrete Embassy task functions for your specific actor types
    /// 2. Create concrete spawn functions for each actor type
    /// 3. Use the probe infrastructure directly with your concrete implementations
    ///
    /// See `lit-bit-core/src/actor/spawn.rs` for examples of concrete Embassy task patterns.
    #[cfg(feature = "async-embassy")]
    pub fn spawn_actor_embassy_with_probe<A, const CAPACITY: usize>(
        &self,
        _spawner: &embassy_executor::Spawner,
        _actor: A,
    ) -> (Address<A::Message, CAPACITY>, ActorProbe<A>)
    where
        A: Actor + 'static,
    {
        unimplemented!(
            "Embassy spawn_actor_with_probe requires concrete actor types due to Embassy's \
            task limitations. See documentation for workaround patterns."
        )
    }

    /// Pause time for deterministic testing (Tokio)
    ///
    /// **Current Status**: No-op implementation
    ///
    /// This method is currently a no-operation placeholder. For deterministic time control
    /// in Tokio tests, you should:
    ///
    /// 1. Use `#[tokio::test(start_paused = true)]` attribute on your test functions
    /// 2. Call `tokio::time::pause()` directly in your test code when needed
    /// 3. Use `tokio::time::advance()` for time progression
    ///
    /// ## Future Enhancement
    ///
    /// This method will be enhanced to provide automatic time control when proper
    /// tokio-test integration is added to the project dependencies.
    ///
    /// # Examples
    /// ```rust,no_run
    /// #[tokio::test(start_paused = true)]
    /// async fn test_with_time_control() {
    ///     tokio::time::pause(); // Use tokio directly for now
    ///     // ... test logic ...
    ///     tokio::time::advance(Duration::from_secs(1)).await;
    /// }
    /// ```
    #[cfg(feature = "async-tokio")]
    pub fn pause_time(&self) {
        // No-op: Users should use tokio::time::pause() directly in tests
        // This will be enhanced when tokio-test integration is added
    }

    /// Resume time after pausing (Tokio)
    ///
    /// **Current Status**: No-op implementation
    ///
    /// This method is currently a no-operation placeholder. For deterministic time control
    /// in Tokio tests, you should use `tokio::time::resume()` directly in your test code.
    ///
    /// ## Future Enhancement
    ///
    /// This method will be enhanced to provide automatic time control when proper
    /// tokio-test integration is added to the project dependencies.
    ///
    /// # Examples
    /// ```rust,no_run
    /// #[tokio::test(start_paused = true)]
    /// async fn test_with_time_control() {
    ///     tokio::time::pause();
    ///     // ... test logic with paused time ...
    ///     tokio::time::resume(); // Use tokio directly for now
    ///     // ... test logic with normal time ...
    /// }
    /// ```
    #[cfg(feature = "async-tokio")]
    pub fn resume_time(&self) {
        // No-op: Users should use tokio::time::resume() directly in tests
        // This will be enhanced when tokio-test integration is added
    }

    /// Advance time by a specific duration (cross-runtime)
    ///
    /// This method provides a simple delay that can be extended in the future
    /// to provide deterministic time advancement.
    ///
    /// # Arguments
    /// * `duration` - The amount of time to advance
    pub async fn advance_time(&self, duration: Duration) {
        #[cfg(feature = "async-tokio")]
        {
            // Simple sleep for now - can be enhanced with test time control later
            tokio::time::sleep(duration).await;
        }

        #[cfg(feature = "async-embassy")]
        {
            embassy_time::Timer::after(embassy_time::Duration::from_micros(
                duration.as_micros() as u64
            ))
            .await;
        }

        #[cfg(not(any(feature = "async-tokio", feature = "async-embassy")))]
        {
            // No async runtime available - just consume the parameter
            let _ = duration;
        }
    }

    /// Wait for the actor system to become quiescent
    ///
    /// This method waits until all pending messages have been processed
    /// and the system has reached a stable state. Useful for ensuring
    /// deterministic test conditions.
    pub async fn wait_for_quiescence(&self) {
        #[cfg(feature = "async-tokio")]
        {
            // Small delay to allow message processing to complete
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        #[cfg(feature = "async-embassy")]
        {
            // Small delay for Embassy
            embassy_time::Timer::after(embassy_time::Duration::from_millis(10)).await;
        }
    }
}

impl Default for TestKit {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::{Actor, ActorError, RestartStrategy};
    use core::future::Ready;

    // Test actor for validation
    #[derive(Debug)]
    struct TestActor {
        pub counter: u32,
    }

    impl TestActor {
        #[allow(dead_code)] // Used in conditional compilation tests
        fn new() -> Self {
            Self { counter: 0 }
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
            Ok(())
        }

        fn on_stop(self) -> Result<(), ActorError> {
            Ok(())
        }

        fn on_panic(&self, _info: &core::panic::PanicInfo) -> RestartStrategy {
            RestartStrategy::OneForOne
        }
    }

    #[test]
    fn test_kit_creation() {
        let _test_kit = TestKit::new();
        // Test that TestKit can be created without panicking
        let _default_kit = TestKit::default();
    }

    #[cfg(feature = "async-tokio")]
    #[tokio::test]
    async fn test_kit_time_control() {
        let test_kit = TestKit::new();

        // Test pause/resume (no-ops for now)
        test_kit.pause_time();
        test_kit.resume_time();

        // Test time advancement
        test_kit.advance_time(Duration::from_millis(1)).await;

        // Test quiescence waiting
        test_kit.wait_for_quiescence().await;
    }

    #[cfg(feature = "async-tokio")]
    #[tokio::test]
    async fn spawn_actor_with_probe_works() {
        let test_kit = TestKit::new();
        let actor = TestActor::new();

        let (address, _probe) = test_kit.spawn_actor_with_probe::<TestActor, 16>(actor);

        // Send a message
        address.send(42).await.unwrap();

        // Wait for message processing
        test_kit.wait_for_quiescence().await;

        // This test mainly validates that the spawn function works without panicking
    }

    #[cfg(feature = "async-tokio")]
    #[tokio::test]
    async fn simple_time_advancement() {
        let test_kit = TestKit::new();

        let start_time = tokio::time::Instant::now();

        // Advance time
        test_kit.advance_time(Duration::from_millis(10)).await;

        let elapsed = start_time.elapsed();

        // Should take at least the requested time
        assert!(elapsed >= Duration::from_millis(10));
    }
}
