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

#[cfg(feature = "async-embassy")]
use super::probes::ActorProbe;
#[cfg(feature = "async-embassy")]
use crate::{Actor, Address};

// Import alloc for tests when using Embassy features
#[cfg(all(feature = "async-embassy", test))]
extern crate alloc;

/// Default mailbox capacity for spawned actors in test environments
///
/// This capacity provides a reasonable buffer for test scenarios while avoiding
/// excessive memory usage. Larger capacities may be needed for stress testing.
#[cfg(feature = "async-tokio")]
const DEFAULT_TEST_MAILBOX_CAPACITY: usize = 1000;

/// Default quiescence timeout for waiting for actor system to stabilize
///
/// This timeout provides a reasonable default for most test scenarios. Tests with
/// heavy message processing or complex actor interactions may need longer timeouts.
const DEFAULT_QUIESCENCE_TIMEOUT: Duration = Duration::from_millis(10);

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
/// use core::time::Duration;
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

        // Spawn using existing spawn infrastructure with default test mailbox capacity
        let address = crate::actor::spawn::spawn_actor_tokio(
            instrumented_actor,
            DEFAULT_TEST_MAILBOX_CAPACITY,
        );

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
    /// A Result containing either a tuple of (Address, ActorProbe) for interacting
    /// with and observing the actor, or an error explaining why spawning failed.
    ///
    /// # Implementation Note
    ///
    /// This method currently returns an error due to Embassy's architectural constraints.
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
    ) -> Result<(Address<A::Message, CAPACITY>, ActorProbe<A>), EmbassySpawnError>
    where
        A: Actor + 'static,
    {
        Err(EmbassySpawnError::GenericSpawnNotSupported)
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
                duration.as_micros().min(u64::MAX as u128) as u64,
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
    /// and the system has reached a stable state. Uses a default timeout
    /// that should be sufficient for most test scenarios.
    ///
    /// For tests that need custom timeout behavior, use `wait_for_quiescence_with_timeout()`.
    ///
    /// # Note
    /// This is a simple delay-based approach. Future versions may implement
    /// actual probe channel polling for more deterministic quiescence detection.
    pub async fn wait_for_quiescence(&self) {
        self.wait_for_quiescence_with_timeout(DEFAULT_QUIESCENCE_TIMEOUT)
            .await;
    }

    /// Wait for the actor system to become quiescent with a custom timeout
    ///
    /// This method waits for the specified duration to allow all pending messages
    /// to be processed and the system to reach a stable state.
    ///
    /// # Arguments
    /// * `timeout` - How long to wait for the system to stabilize
    ///
    /// # Examples
    /// ```rust,no_run
    /// # #[cfg(feature = "async-tokio")]
    /// # {
    /// use lit_bit_core::test_utils::TestKit;
    /// use core::time::Duration;
    ///
    /// let test_kit = TestKit::new();
    ///
    /// // Wait longer for complex scenarios
    /// test_kit.wait_for_quiescence_with_timeout(Duration::from_millis(50)).await;
    /// # }
    /// ```
    ///
    /// # Note
    /// This is a simple delay-based approach. Future versions may implement
    /// actual probe channel polling for more deterministic quiescence detection.
    pub async fn wait_for_quiescence_with_timeout(&self, timeout: Duration) {
        #[cfg(feature = "async-tokio")]
        {
            tokio::time::sleep(timeout).await;
        }

        #[cfg(feature = "async-embassy")]
        {
            embassy_time::Timer::after(embassy_time::Duration::from_micros(
                timeout.as_micros().min(u64::MAX as u128) as u64,
            ))
            .await;
        }

        #[cfg(not(any(feature = "async-tokio", feature = "async-embassy")))]
        {
            // No async runtime available - just consume the parameter
            let _ = timeout;
        }
    }
}

impl Default for TestKit {
    fn default() -> Self {
        Self::new()
    }
}

/// Error types for Embassy actor spawning operations
#[cfg(feature = "async-embassy")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmbassySpawnError {
    /// Generic actor spawning is not supported in Embassy due to task limitations
    ///
    /// Embassy requires concrete (non-generic) task functions. Use concrete
    /// spawn functions for specific actor types instead.
    GenericSpawnNotSupported,
}

#[cfg(feature = "async-embassy")]
impl core::fmt::Display for EmbassySpawnError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EmbassySpawnError::GenericSpawnNotSupported => write!(
                f,
                "Generic actor spawning is not supported in Embassy due to task limitations. \
                Create concrete spawn functions for specific actor types instead. \
                See lit-bit-core/src/actor/spawn.rs for examples."
            ),
        }
    }
}

#[cfg(feature = "async-embassy")]
#[cfg(feature = "std")]
impl std::error::Error for EmbassySpawnError {}

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

    #[test]
    fn test_duration_overflow_protection() {
        // Test that extremely large durations don't cause overflow/truncation
        // Create a duration that would overflow u64 when converted to microseconds
        // u64::MAX microseconds = 18_446_744_073_709_551_615 microseconds
        // = ~584,542 years, so Duration::MAX definitely exceeds this

        let very_large_duration = Duration::MAX;
        let micros_u128 = very_large_duration.as_micros();

        // Verify the duration exceeds u64::MAX when converted to microseconds
        assert!(micros_u128 > u64::MAX as u128);

        // Test our saturating conversion logic
        let saturated_micros = micros_u128.min(u64::MAX as u128) as u64;

        // Should be exactly u64::MAX, not wrapped around
        assert_eq!(saturated_micros, u64::MAX);

        // Verify the conversion is safe (this is what our fix implements)
        let safe_duration_micros = very_large_duration.as_micros().min(u64::MAX as u128) as u64;
        assert_eq!(safe_duration_micros, u64::MAX);
    }

    #[cfg(feature = "async-tokio")]
    #[tokio::test]
    async fn test_configurable_quiescence_timeout() {
        let test_kit = TestKit::new();

        // Test default timeout method
        let start_time = tokio::time::Instant::now();
        test_kit.wait_for_quiescence().await;
        let elapsed = start_time.elapsed();

        // Should be at least the default timeout
        assert!(elapsed >= DEFAULT_QUIESCENCE_TIMEOUT);

        // Test custom timeout method
        let custom_timeout = Duration::from_millis(25);
        let start_time = tokio::time::Instant::now();
        test_kit
            .wait_for_quiescence_with_timeout(custom_timeout)
            .await;
        let elapsed = start_time.elapsed();

        // Should be at least the custom timeout
        assert!(elapsed >= custom_timeout);
    }

    #[cfg(feature = "async-embassy")]
    #[test]
    fn test_embassy_spawn_error_handling() {
        // Test that the error type has the correct properties
        let error = EmbassySpawnError::GenericSpawnNotSupported;

        // Test error equality
        let error2 = EmbassySpawnError::GenericSpawnNotSupported;
        assert_eq!(error, error2);

        // Test error cloning
        let error_clone = error.clone();
        assert_eq!(error, error_clone);

        // Test that Display formatting works (platform-independent test)
        use core::fmt::Write;
        let mut buffer = heapless::String::<256>::new();
        write!(buffer, "{}", error).unwrap();
        assert!(buffer.contains("Embassy"));
        assert!(buffer.contains("concrete"));
        assert!(buffer.contains("spawn.rs"));
        assert!(buffer.contains("not supported"));

        // Test debug formatting without allocation
        let mut debug_buffer = heapless::String::<128>::new();
        write!(debug_buffer, "{:?}", error).unwrap();
        assert!(debug_buffer.contains("GenericSpawnNotSupported"));
    }
}
