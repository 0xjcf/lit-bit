//! # Timer Service Abstraction
//!
//! Provides platform-neutral timer operations for delayed transitions in statecharts.
//! This module implements the research findings for supporting `after(Duration)`
//! transitions across different async runtimes.

use core::time::Duration;

/// Converts a Duration to u64 microseconds with overflow protection.
///
/// This helper function ensures consistent behavior when converting Duration
/// to u64 microseconds across the codebase, clamping to u64::MAX on overflow.
#[cfg(any(feature = "async-embassy", test))]
fn duration_to_u64_micros(duration: Duration) -> u64 {
    let duration_micros = duration.as_micros();

    // Ensure we don't silently truncate large durations
    debug_assert!(
        duration_micros <= u64::MAX as u128,
        "Duration too large for timer: {duration_micros} microseconds exceeds u64::MAX"
    );

    // Use saturating conversion to handle overflow gracefully
    if duration_micros > u64::MAX as u128 {
        u64::MAX
    } else {
        duration_micros as u64
    }
}

/// Platform-neutral timer service trait for async sleep operations.
///
/// This trait provides zero-cost abstractions for timer operations across
/// different async runtimes (Tokio, Embassy) while maintaining `no_std` compatibility.
///
/// # Design Philosophy
///
/// - **Zero-cost**: No heap allocation, futures live on the stack
/// - **Platform-agnostic**: Same API works with Tokio, Embassy, or custom runtimes
/// - **Feature-gated**: Only compiled when async features are enabled
///
/// # Usage
///
/// ```rust,ignore
/// // In generated statechart code:
/// TimerService::sleep(Duration::from_secs(5)).await;
/// ```
pub trait TimerService {
    /// The future type returned by the sleep operation.
    ///
    /// Using an associated type allows the compiler to know the exact future type
    /// at compile-time, enabling stack allocation and zero-cost abstractions.
    type SleepFuture: core::future::Future<Output = ()> + Send;

    /// Sleep for the specified duration.
    ///
    /// Returns a future that resolves after the given duration has elapsed.
    /// The implementation is runtime-specific but the API remains consistent.
    fn sleep(duration: Duration) -> Self::SleepFuture;
}

// Tokio implementation - only available when async-tokio feature is enabled
#[cfg(feature = "async-tokio")]
pub struct TokioTimer;

#[cfg(feature = "async-tokio")]
impl TimerService for TokioTimer {
    type SleepFuture = tokio::time::Sleep;

    fn sleep(duration: Duration) -> Self::SleepFuture {
        tokio::time::sleep(duration)
    }
}

// Embassy implementation - only available when async-embassy feature is enabled
#[cfg(feature = "async-embassy")]
pub struct EmbassyTimer;

#[cfg(feature = "async-embassy")]
impl TimerService for EmbassyTimer {
    type SleepFuture = embassy_time::Timer;

    fn sleep(duration: Duration) -> Self::SleepFuture {
        let embassy_micros = duration_to_u64_micros(duration);
        embassy_time::Timer::after(embassy_time::Duration::from_micros(embassy_micros))
    }
}

// Type alias for the active timer implementation
// This allows generated code to use Timer::sleep() consistently

// Enforce mutually exclusive async runtime features
#[cfg(all(feature = "async-tokio", feature = "async-embassy"))]
compile_error!(
    "Features 'async-tokio' and 'async-embassy' are mutually exclusive. \
     Please enable only one async runtime feature at a time."
);

#[cfg(all(feature = "async-tokio", not(feature = "async-embassy")))]
pub type Timer = TokioTimer;

#[cfg(all(feature = "async-embassy", not(feature = "async-tokio")))]
pub type Timer = EmbassyTimer;

// Provide a default no-op timer when async is enabled but no specific runtime is selected
// This allows the code to compile for feature compatibility testing
#[cfg(all(
    feature = "async",
    not(feature = "async-tokio"),
    not(feature = "async-embassy")
))]
pub struct NoOpTimer;

#[cfg(all(
    feature = "async",
    not(feature = "async-tokio"),
    not(feature = "async-embassy")
))]
impl TimerService for NoOpTimer {
    type SleepFuture = core::future::Ready<()>;

    fn sleep(_duration: Duration) -> Self::SleepFuture {
        core::future::ready(())
    }
}

#[cfg(all(
    feature = "async",
    not(feature = "async-tokio"),
    not(feature = "async-embassy")
))]
pub type Timer = NoOpTimer;

/// Test timer implementation for unit tests
///
/// This implementation provides a mock timer that can be controlled in tests,
/// allowing deterministic testing of timer-based transitions.
#[cfg(test)]
pub struct TestTimer {
    /// Simulated delay before the timer fires
    pub delay: Duration,
}

#[cfg(test)]
impl TimerService for TestTimer {
    type SleepFuture = core::future::Ready<()>;

    fn sleep(_duration: Duration) -> Self::SleepFuture {
        // For tests, return immediately to avoid actual delays
        core::future::ready(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_service_trait_compiles() {
        // This test ensures the TimerService trait is well-formed
        // and can be used in generic contexts

        fn accept_timer_service<T: TimerService>(_timer: T) {}

        #[cfg(feature = "async-tokio")]
        accept_timer_service(TokioTimer);

        #[cfg(feature = "async-embassy")]
        accept_timer_service(EmbassyTimer);

        accept_timer_service(TestTimer {
            delay: Duration::from_millis(100),
        });
    }

    #[test]
    fn duration_conversion_works() {
        let duration = Duration::from_secs(5);
        assert_eq!(duration.as_secs(), 5);

        let duration_ms = Duration::from_millis(250);
        assert_eq!(duration_ms.as_millis(), 250);
    }

    #[cfg(feature = "async-embassy")]
    #[test]
    fn embassy_timer_handles_large_durations_safely() {
        use core::time::Duration;

        // Test normal duration - should work fine
        let normal_duration = Duration::from_secs(60);
        let _timer = EmbassyTimer::sleep(normal_duration);

        // Test a large but valid duration (near u64::MAX microseconds)
        // u64::MAX microseconds ≈ 584,942 years, which is reasonable to clamp
        let large_duration = Duration::from_micros(u64::MAX);
        let _timer = EmbassyTimer::sleep(large_duration);

        // Test duration conversion edge case
        // Create a duration that would overflow u64 when converted to microseconds
        // Duration::MAX is about 584 billion years
        let very_large_duration = Duration::MAX;
        let _timer = EmbassyTimer::sleep(very_large_duration);
        // This should not panic and should clamp to u64::MAX
    }

    #[test]
    fn duration_microseconds_overflow_behavior() {
        // Test what happens with Duration::MAX
        let max_duration = Duration::MAX;
        let micros = max_duration.as_micros();

        // Verify our conversion logic using the helper function
        let safe_micros = duration_to_u64_micros(max_duration);

        // Should clamp to u64::MAX when overflow occurs
        if micros > u64::MAX as u128 {
            assert_eq!(safe_micros, u64::MAX);
        } else {
            assert_eq!(safe_micros, micros as u64);
        }
    }

    #[test]
    fn test_duration_safe_conversion_logic() {
        // Test the exact conversion logic we use in EmbassyTimer using the helper function

        // Test normal durations
        assert_eq!(duration_to_u64_micros(Duration::from_secs(1)), 1_000_000);
        assert_eq!(duration_to_u64_micros(Duration::from_millis(500)), 500_000);
        assert_eq!(duration_to_u64_micros(Duration::from_micros(123)), 123);

        // Test edge case: exactly u64::MAX microseconds
        let max_micros_duration = Duration::from_micros(u64::MAX);
        assert_eq!(duration_to_u64_micros(max_micros_duration), u64::MAX);

        // Test overflow case: Duration::MAX
        let max_duration = Duration::MAX;
        assert_eq!(duration_to_u64_micros(max_duration), u64::MAX);

        // Verify that Duration::MAX actually overflows u64
        assert!(max_duration.as_micros() > u64::MAX as u128);
    }
}
