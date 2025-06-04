//! Minimal Actor trait and supervision primitives for the actor framework.

#![allow(dead_code)]

use core::panic::PanicInfo;
#[cfg(not(feature = "async-tokio"))]
use static_cell::StaticCell;

// Platform-dual string support for panic information
#[cfg(any(feature = "std", feature = "alloc"))]
type ActorString = alloc::string::String;

#[cfg(not(any(feature = "std", feature = "alloc")))]
type ActorString = heapless::String<128>; // Fixed-size string for no_std

// Import alloc when available for ActorString
#[cfg(any(feature = "std", feature = "alloc"))]
extern crate alloc;

// Box import for supervision messages
#[cfg(any(feature = "std", feature = "alloc"))]
use alloc::boxed::Box;

// Type alias for platform-dual error boxing
#[cfg(any(feature = "std", feature = "alloc"))]
type BoxedActorError = Box<ActorError>;

#[cfg(not(any(feature = "std", feature = "alloc")))]
type BoxedActorError = ActorError;

/// Error type for actor lifecycle and supervision hooks.
///
/// Enhanced with panic-specific details for Task 5.4 implementation.
/// Based on research from Actix, Ractor, and Bastion panic handling patterns.
// Supervision needs full context — allow large error for comprehensive panic information
#[allow(clippy::result_large_err)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActorError {
    /// Actor failed to start properly
    StartupFailure,
    /// Actor failed to shutdown cleanly
    ShutdownFailure,
    /// Actor panicked during message processing
    ///
    /// Contains optional panic details for debugging and supervision decisions.
    /// Platform-specific panic capture utilities populate these fields.
    Panic {
        /// Panic message extracted from the panic payload, if available
        message: Option<ActorString>,
        /// Actor identifier for supervision context, if available
        actor_id: Option<ActorString>,
    },
    /// Actor mailbox was closed unexpectedly
    MailboxClosed,
    /// Actor operation timed out
    Timeout,
    /// Supervision system failure
    SupervisionFailure(ActorString),
    /// Custom error with static message for no_std compatibility
    Custom(&'static str),
}

/// Restart strategy for actor supervision (OTP-inspired).
///
/// Enhanced with OTP-style restart policies and deterministic backoff strategies
/// for comprehensive panic-aware supervision integration (Task 5.4 Phase 2).
///
/// ## OTP-Style Policies
/// - **Permanent**: Always restart on any termination (normal or abnormal)
/// - **Transient**: Restart only on abnormal termination (panic/error), not on normal exit
/// - **Temporary**: Never restart, let the actor die permanently
///
/// ## Restart Patterns
/// - **OneForOne**: Restart only the failed child (classic isolation)
/// - **OneForAll**: Restart all sibling actors (shared state recovery)
/// - **RestForOne**: Restart failed child and all children started after it (dependency chain)
///
/// ## Advanced Policies
/// - **Escalate**: Don't restart, escalate failure to parent supervisor
/// - **Never**: Explicit no-restart policy (clearer than Temporary)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartStrategy {
    // Classic restart patterns
    /// Restart only this actor (default, classic isolation pattern)
    OneForOne,
    /// Restart all sibling actors (shared state recovery pattern)
    OneForAll,
    /// Restart this and all actors started after it (dependency chain pattern)
    RestForOne,

    // OTP-style restart policies
    /// Always restart on any termination (normal or abnormal)
    /// Use for critical actors that must always be running
    Permanent,
    /// Restart only on abnormal termination (panic/error), not on normal exit
    /// Use for workers that may exit normally but should recover from crashes
    Transient,
    /// Never restart, let the actor die permanently
    /// Use for one-shot tasks or actors that shouldn't be automatically restarted
    Temporary,

    // Advanced supervision policies
    /// Don't restart, escalate failure to parent supervisor
    /// Use when failure indicates a larger system issue requiring higher-level intervention
    Escalate,
    /// Explicit no-restart policy (clearer than Temporary)
    /// Use when you want to explicitly document that restart is not desired
    Never,
}

/// Restart intensity configuration for supervision with deterministic backoff.
///
/// Controls restart rate limiting and backoff behavior to prevent crash loops
/// while maintaining deterministic timing for embedded environments.
#[derive(Debug, Clone)]
pub struct RestartIntensity {
    /// Maximum number of restarts within the time window before escalating
    pub max_restarts: u32,
    /// Time window for restart counting (in milliseconds)
    pub restart_window_ms: u64,
    /// Backoff strategy to apply between restart attempts
    pub backoff_strategy: BackoffStrategy,
}

impl Default for RestartIntensity {
    fn default() -> Self {
        Self {
            max_restarts: 5,
            restart_window_ms: 60_000, // 60 seconds
            backoff_strategy: BackoffStrategy::Exponential {
                base_delay_ms: 100,
                max_delay_ms: 30_000, // 30 seconds max
            },
        }
    }
}

/// Deterministic backoff strategy for restart attempts.
///
/// All strategies provide deterministic delays based on failure count,
/// ensuring predictable behavior in embedded and real-time systems.
#[derive(Debug, Clone)]
pub enum BackoffStrategy {
    /// No delay between restart attempts
    Immediate,
    /// Linear backoff: delay = base_delay_ms * failure_count
    Linear {
        /// Base delay in milliseconds
        base_delay_ms: u64,
    },
    /// Exponential backoff: delay = base_delay_ms * 2^failure_count (capped at max)
    Exponential {
        /// Base delay in milliseconds
        base_delay_ms: u64,
        /// Maximum delay in milliseconds (prevents infinite growth)
        max_delay_ms: u64,
    },
    /// Fixed delay regardless of failure count
    Fixed {
        /// Fixed delay in milliseconds
        delay_ms: u64,
    },
}

/// Trait for analyzing panics and determining supervision actions.
///
/// Provides extensible panic analysis for custom supervision policies.
/// Users can implement this trait to customize restart decisions based on
/// panic details, actor context, or system state.
pub trait PanicAnalyzer {
    /// Analyze panic and determine if actor should be restarted
    ///
    /// # Arguments
    /// * `child_id` - Identifier of the failed child (for context-aware decisions)
    /// * `error` - The actor error that caused the failure
    ///
    /// # Returns
    /// `true` if the actor should be restarted, `false` otherwise
    fn should_restart(&self, child_id: &dyn core::fmt::Debug, error: &ActorError) -> bool;

    /// Calculate backoff delay for restart (deterministic)
    ///
    /// # Arguments
    /// * `failure_count` - Number of consecutive failures (starting from 1)
    /// * `strategy` - Backoff strategy to apply
    ///
    /// # Returns
    /// Delay in milliseconds before attempting restart
    fn calculate_backoff_delay(&self, failure_count: u32, strategy: &BackoffStrategy) -> u64;

    /// Determine if supervisor should escalate failure
    ///
    /// # Arguments
    /// * `child_id` - Identifier of the failed child
    /// * `failure_count` - Number of consecutive failures
    /// * `intensity` - Restart intensity configuration
    ///
    /// # Returns
    /// `true` if failure should be escalated to parent supervisor
    fn should_escalate(
        &self,
        child_id: &dyn core::fmt::Debug,
        failure_count: u32,
        intensity: &RestartIntensity,
    ) -> bool;
}

/// Default implementation of PanicAnalyzer following OTP patterns.
///
/// Provides sensible defaults for most supervision scenarios:
/// - Restarts on panics and custom errors
/// - Does not restart on clean shutdown failures
/// - Uses deterministic backoff calculations
/// - Escalates when restart intensity is exceeded
#[derive(Debug, Default)]
pub struct DefaultPanicAnalyzer;

impl PanicAnalyzer for DefaultPanicAnalyzer {
    fn should_restart(&self, _child_id: &dyn core::fmt::Debug, error: &ActorError) -> bool {
        match error {
            ActorError::Panic { .. } => true,
            ActorError::Custom(_) => true,
            ActorError::StartupFailure => true,
            ActorError::MailboxClosed => true,
            ActorError::Timeout => true,
            ActorError::SupervisionFailure(_) => false,
            ActorError::ShutdownFailure => false, // Don't restart on clean shutdown failure
        }
    }

    fn calculate_backoff_delay(&self, failure_count: u32, strategy: &BackoffStrategy) -> u64 {
        match strategy {
            BackoffStrategy::Immediate => 0,
            BackoffStrategy::Linear { base_delay_ms } => {
                base_delay_ms.saturating_mul(failure_count as u64)
            }
            BackoffStrategy::Exponential {
                base_delay_ms,
                max_delay_ms,
            } => {
                let delay = base_delay_ms
                    .saturating_mul(2_u64.saturating_pow(failure_count.saturating_sub(1)));
                delay.min(*max_delay_ms)
            }
            BackoffStrategy::Fixed { delay_ms } => *delay_ms,
        }
    }

    fn should_escalate(
        &self,
        _child_id: &dyn core::fmt::Debug,
        failure_count: u32,
        intensity: &RestartIntensity,
    ) -> bool {
        failure_count >= intensity.max_restarts
    }
}

/// Supervisor message for communication between supervisor and child actors.
///
/// This message type enables the OTP-style supervision patterns described in the research.
/// Supervisors can receive notifications about child lifecycle events and react accordingly.
/// Enhanced with detailed error reporting for Task 5.4 panic handling and hierarchical escalation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)] // Error information is important for supervision decisions
pub enum SupervisorMessage<ChildId = u32> {
    /// Child actor has started successfully
    ChildStarted { id: ChildId },
    /// Child actor has stopped gracefully
    ChildStopped { id: ChildId },
    /// Child actor has panicked or failed
    ///
    /// Enhanced with detailed error information including panic details.
    /// This enables supervisors to make informed restart decisions based on failure type.
    /// Error is boxed to reduce enum size.
    ChildPanicked { id: ChildId, error: BoxedActorError },
    /// Request to start a new child actor
    StartChild { id: ChildId },
    /// Request to stop a child actor
    StopChild { id: ChildId },
    /// Request to restart a child actor
    RestartChild { id: ChildId },

    // Enhanced hierarchical supervision messages
    /// Child supervisor has escalated a failure (hierarchical supervision)
    ///
    /// Sent when a child supervisor cannot handle a failure and escalates it upward.
    /// Enables multi-level supervision trees with controlled fault propagation.
    ChildEscalated {
        /// ID of the supervisor that escalated
        supervisor_id: ChildId,
        /// ID of the original failed child within that supervisor
        failed_child_id: ChildId,
        /// The original error that triggered the escalation
        error: BoxedActorError,
    },
}

/// Supervisor trait for managing child actors with restart strategies.
///
/// Implements OTP-style supervision patterns as described in the research document.
/// Supervisors can monitor children and apply restart strategies when failures occur.
///
/// ## Design Principles
///
/// - **Platform-agnostic**: Works with both Tokio (JoinHandle monitoring) and Embassy (message signaling)
/// - **Zero-allocation**: Uses fixed-size child lists in `no_std` environments
/// - **Deterministic**: Failure notifications are processed as regular messages
/// - **Restart strategies**: Supports OneForOne, OneForAll, and RestForOne patterns
///
/// ## Usage
///
/// ```rust,no_run
/// use lit_bit_core::actor::{Supervisor, SupervisorMessage, RestartStrategy};
/// use lit_bit_core::Address;
/// use heapless::Vec;
///
/// struct MySupervisor<ChildMsg> {
///     children: Vec<(u32, Address<ChildMsg, 8>), 4>,
/// }
///
/// impl<ChildMsg> Supervisor for MySupervisor<ChildMsg> {
///     type ChildId = u32;
///     
///     fn on_child_failure(&mut self, child_id: u32) -> RestartStrategy {
///         // Restart only the failed child
///         RestartStrategy::OneForOne
///     }
/// }
/// ```
pub trait Supervisor {
    /// Type used to identify child actors
    type ChildId: Clone + PartialEq + core::fmt::Debug;

    /// Called when a child actor fails or panics.
    ///
    /// The supervisor should return the appropriate restart strategy to handle the failure.
    /// The framework will then apply the strategy by restarting the appropriate actors.
    ///
    /// # Arguments
    /// * `child_id` - Identifier of the failed child actor
    ///
    /// # Returns
    /// The restart strategy to apply for this failure
    fn on_child_failure(&mut self, child_id: Self::ChildId) -> RestartStrategy;

    /// Called when a child actor starts successfully.
    ///
    /// Default implementation does nothing. Override to track child state or perform
    /// additional setup after child startup.
    ///
    /// # Arguments
    /// * `child_id` - Identifier of the child actor that started
    fn on_child_started(&mut self, _child_id: Self::ChildId) {}

    /// Called when a child actor stops gracefully.
    ///
    /// Default implementation does nothing. Override to track child state or perform
    /// cleanup after child shutdown.
    ///
    /// # Arguments
    /// * `child_id` - Identifier of the child actor that stopped
    fn on_child_stopped(&mut self, _child_id: Self::ChildId) {}
}

/// Batch processing trait for high-throughput message handling.
///
/// Implements zero-allocation message batching as described in the research document.
/// Actors can opt into batch processing to improve throughput by processing multiple
/// queued messages in a single wake-up cycle.
///
/// ## Design Principles
///
/// - **Zero-allocation**: Uses existing queue memory, no additional buffers
/// - **Optional**: Actors can implement either `Actor` or `BatchActor` or both
/// - **Deterministic**: Messages are processed in FIFO order within each batch
/// - **Bounded**: Configurable batch size limits prevent monopolizing the executor
/// - **Platform-agnostic**: Works with both heapless and Tokio channels
///
/// ## Performance Benefits
///
/// - Reduced context switching overhead (fewer executor wake-ups)
/// - Better CPU cache locality (processing related messages together)
/// - Amortized per-message overhead across the batch
/// - Higher overall throughput for high-frequency message scenarios
///
/// ## Usage
///
/// ```rust,no_run
/// use lit_bit_core::actor::BatchActor;
///
/// struct HighThroughputActor {
///     processed_count: u32,
/// }
///
/// impl BatchActor for HighThroughputActor {
///     type Message = u32;
///     type Future<'a> = core::future::Ready<()> where Self: 'a;
///
///     fn handle_batch(&mut self, messages: &[Self::Message]) -> Self::Future<'_> {
///         // Process all messages in the batch
///         for &msg in messages {
///             self.processed_count += msg;
///         }
///         core::future::ready(())
///     }
///
///     fn max_batch_size(&self) -> usize {
///         16 // Process up to 16 messages per batch
///     }
/// }
/// ```
pub trait BatchActor: Send {
    /// The message type this actor handles
    type Message: Send + 'static;

    /// The future type returned by `handle_batch()` - uses GATs for zero-cost async
    type Future<'a>: core::future::Future<Output = ()> + Send + 'a
    where
        Self: 'a;

    /// Handle a batch of messages asynchronously.
    ///
    /// This method is called with a slice of pending messages from the actor's mailbox.
    /// The implementation should process all messages in the slice before returning.
    ///
    /// ## Atomicity Guarantee
    ///
    /// The actor runtime guarantees that:
    /// - Only one call to `handle_batch()` is active at a time per actor
    /// - All messages in the batch are processed before dequeuing new messages
    /// - The batch slice contains messages in FIFO order
    ///
    /// ## Batch Size
    ///
    /// The actual batch size depends on:
    /// - Number of messages currently queued (up to `max_batch_size()`)
    /// - Runtime batch size limits (to maintain fairness with other actors)
    /// - Platform-specific queue draining capabilities
    ///
    /// ## Examples
    ///
    /// ### Sync-style batch handler
    /// ```rust,no_run
    /// # use lit_bit_core::actor::BatchActor;
    /// # struct MyActor;
    /// # impl BatchActor for MyActor {
    /// #     type Message = u32;
    /// #     type Future<'a> = std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> where Self: 'a;
    /// #     fn max_batch_size(&self) -> usize { 32 }
    /// #     fn handle_batch(&mut self, messages: &[u32]) -> Self::Future<'_> {
    /// Box::pin(async move {
    ///     for &value in messages {
    ///         // self.accumulator += value; // Synchronous processing
    ///     }
    /// })
    /// #     }
    /// # }
    /// ```
    ///
    /// ### Async batch handler with I/O
    /// ```rust,no_run
    /// # use lit_bit_core::actor::BatchActor;
    /// # struct MyActor;
    /// # struct IoRequest;
    /// # impl BatchActor for MyActor {
    /// #     type Message = IoRequest;
    /// #     type Future<'a> = std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> where Self: 'a;
    /// #     fn max_batch_size(&self) -> usize { 32 }
    /// #     fn handle_batch(&mut self, messages: &[IoRequest]) -> Self::Future<'_> {
    /// Box::pin(async move {
    ///     for request in messages {
    ///         // let result = self.io_device.process(request).await;
    ///         // self.handle_result(result);
    ///     }
    /// })
    /// #     }
    /// # }
    #[must_use]
    fn handle_batch(&mut self, messages: &[Self::Message]) -> Self::Future<'_>;

    /// Maximum number of messages to process in a single batch.
    ///
    /// This setting helps balance throughput and fairness:
    /// - **Higher values**: Better throughput for high-frequency messages
    /// - **Lower values**: Better responsiveness and fairness with other actors
    ///
    /// ## Platform Considerations
    ///
    /// - **Embassy**: Lower values (8-32) recommended to avoid starving other tasks
    /// - **Tokio**: Higher values (64-256) acceptable due to work-stealing scheduler
    /// - **Real-time**: Very low values (1-8) for deterministic latency
    ///
    /// ## Default Implementation
    ///
    /// Returns 32 as a reasonable default that balances throughput and fairness.
    fn max_batch_size(&self) -> usize {
        32
    }

    /// Called when the actor starts. Default: Ok(())
    ///
    /// # Errors
    /// Returns `Err(ActorError)` if actor startup fails.
    #[allow(clippy::result_large_err)] // ActorError provides detailed failure information
    fn on_start(&mut self) -> Result<(), ActorError> {
        Ok(())
    }

    /// Called when the actor stops. Default: Ok(())
    ///
    /// # Errors
    /// Returns `Err(ActorError)` if actor shutdown fails.
    #[allow(clippy::result_large_err)] // ActorError provides detailed failure information
    fn on_stop(self) -> Result<(), ActorError>
    where
        Self: Sized,
    {
        Ok(())
    }

    /// Called if the actor panics. Default: `RestartStrategy::OneForOne`
    fn on_panic(&self, _info: &PanicInfo) -> RestartStrategy {
        RestartStrategy::OneForOne
    }

    /// Called before restart to reset state.
    ///
    /// This hook allows actors to clean up state or perform initialization
    /// when restarted by a supervisor. Called after actor creation but before
    /// the first message is processed.
    ///
    /// # Returns
    /// `Ok(())` if restart preparation succeeds, `Err(ActorError)` if it fails
    #[allow(clippy::result_large_err)] // ActorError provides detailed failure information
    fn on_restart(&mut self) -> Result<(), ActorError> {
        // Default: no special restart logic
        Ok(())
    }
}

/// Core Actor trait using Generic Associated Types (GATs) for zero-cost async.
///
/// This trait provides the foundation for both sync and async actors while maintaining
/// `#![no_std]` compatibility. The GAT-based design allows for stack-allocated futures
/// without heap allocation.
///
/// ## Design Principles
///
/// - **Zero-cost abstraction**: No heap allocation in `no_std` environments
/// - **Deterministic execution**: One message processed at a time per actor
/// - **Platform-agnostic**: Works with Tokio, Embassy, and custom executors
/// - **Backward compatible**: Existing sync code continues to work unchanged
///
/// ## Usage
///
/// ```rust,no_run
/// use lit_bit_core::actor::Actor;
/// use core::future::Ready;
///
/// struct MyActor {
///     counter: u32,
/// }
///
/// impl Actor for MyActor {
///     type Message = u32;
///     type Future<'a> = Ready<()> where Self: 'a;
///
///     fn handle(&mut self, msg: Self::Message) -> Self::Future<'_> {
///         self.counter += msg;
///         // For sync operations, use core::future::ready()
///         core::future::ready(())
///     }
/// }
/// ```
pub trait Actor: Send {
    /// The message type this actor handles
    type Message: Send + 'static;

    /// The future type returned by `handle()` - uses GATs for zero-cost async
    type Future<'a>: core::future::Future<Output = ()> + Send + 'a
    where
        Self: 'a;

    /// Handle a single message asynchronously.
    ///
    /// This method is called for each message received by the actor. The implementation
    /// should process the message and return a future that completes when processing
    /// is done. The actor runtime ensures that only one message is processed at a time,
    /// maintaining deterministic execution.
    ///
    /// ## Atomicity Guarantee
    ///
    /// The actor runtime guarantees that:
    /// - Only one call to `handle()` is active at a time per actor
    /// - No new messages are dequeued until the current future completes
    /// - Actor state is protected during async operations (Actix-style atomicity)
    ///
    /// ## Examples
    ///
    /// ### Sync-style handler (compiles to sync code)
    /// ```rust,no_run
    /// # use lit_bit_core::actor::Actor;
    /// # struct MyActor;
    /// # impl Actor for MyActor {
    /// #     type Message = u32;
    /// #     type Future<'a> = std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> where Self: 'a;
    /// #     fn handle(&mut self, msg: u32) -> Self::Future<'_> {
    /// Box::pin(async move {
    ///     // self.counter += msg; // Synchronous operation
    /// })
    /// #     }
    /// # }
    /// ```
    ///
    /// ### Async handler with I/O
    /// ```rust,no_run
    /// # use lit_bit_core::actor::Actor;
    /// # struct MyActor;
    /// # struct SensorRequest;
    /// # impl Actor for MyActor {
    /// #     type Message = SensorRequest;
    /// #     type Future<'a> = std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> where Self: 'a;
    /// #     fn handle(&mut self, msg: SensorRequest) -> Self::Future<'_> {
    /// Box::pin(async move {
    ///     // let reading = self.sensor.read().await; // Async I/O
    ///     // self.process_reading(reading);
    /// })
    /// #     }
    /// # }
    #[must_use]
    fn handle(&mut self, msg: Self::Message) -> Self::Future<'_>;

    /// Called when the actor starts. Default: Ok(())
    ///
    /// # Errors
    /// Returns `Err(ActorError)` if actor startup fails.
    #[allow(clippy::result_large_err)] // ActorError provides detailed failure information
    fn on_start(&mut self) -> Result<(), ActorError> {
        Ok(())
    }

    /// Called when the actor stops. Default: Ok(())
    ///
    /// # Errors
    /// Returns `Err(ActorError)` if actor shutdown fails.
    #[allow(clippy::result_large_err)] // ActorError provides detailed failure information
    fn on_stop(self) -> Result<(), ActorError>
    where
        Self: Sized,
    {
        Ok(())
    }

    /// Called if the actor panics. Default: `RestartStrategy::OneForOne`
    fn on_panic(&self, _info: &PanicInfo) -> RestartStrategy {
        RestartStrategy::OneForOne
    }

    /// Called before restart to reset state.
    ///
    /// This hook allows actors to clean up state or perform initialization
    /// when restarted by a supervisor. Called after actor creation but before
    /// the first message is processed.
    ///
    /// # Returns
    /// `Ok(())` if restart preparation succeeds, `Err(ActorError)` if it fails
    #[allow(clippy::result_large_err)] // ActorError provides detailed failure information
    fn on_restart(&mut self) -> Result<(), ActorError> {
        // Default: no special restart logic
        Ok(())
    }

    /// Embassy-compatible error-returning message handler.
    ///
    /// This method provides an alternative to `handle()` that returns `Result<(), ActorError>`
    /// instead of just `()`. This enables Embassy actors to signal failures cooperatively
    /// without relying on panic unwinding, which is not available in no_std environments.
    ///
    /// ## Default Implementation
    ///
    /// The default implementation wraps the regular `handle()` method for backward compatibility.
    /// Embassy actors should override this method to provide explicit error handling.
    ///
    /// ## Usage Patterns
    ///
    /// - **Tokio actors**: Can continue using `handle()` with panic recovery
    /// - **Embassy actors**: Should override `handle_safe()` to return errors explicitly
    /// - **Dual-platform actors**: Can override both for optimal platform behavior
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use lit_bit_core::actor::{Actor, ActorError};
    /// # struct MyActor;
    /// impl Actor for MyActor {
    ///     type Message = String;
    ///     type Future<'a> = core::future::Ready<()> where Self: 'a;
    ///
    ///     fn handle(&mut self, _msg: String) -> Self::Future<'_> {
    ///         core::future::ready(())
    ///     }
    ///
    ///     // Embassy-specific error handling
    ///     async fn handle_safe(&mut self, msg: String) -> Result<(), ActorError> {
    ///         if msg == "error" {
    ///             return Err(ActorError::Custom("Deliberate failure"));
    ///         }
    ///         self.handle(msg).await;
    ///         Ok(())
    ///     }
    /// }
    /// ```
    ///
    /// Note: Embassy async trait design choice - suppressing lint for cooperative error handling patterns
    #[allow(async_fn_in_trait)]
    #[allow(clippy::result_large_err)] // ActorError provides detailed failure information
    async fn handle_safe(&mut self, msg: Self::Message) -> Result<(), ActorError> {
        // Default implementation wraps regular handle() for backward compatibility
        self.handle(msg).await;
        Ok(())
    }

    /// Embassy cleanup hook called before restart.
    ///
    /// This method allows actors to perform cleanup operations before being restarted
    /// by a supervisor. Unlike `on_stop()`, this method doesn't consume `self` and
    /// can be called multiple times during an actor's lifetime.
    ///
    /// ## Usage
    ///
    /// - Called by Embassy loop-based restart patterns before state reset
    /// - Allows releasing resources without destroying the actor instance
    /// - Enables graceful resource cleanup in restart scenarios
    ///
    /// ## Default Implementation
    ///
    /// The default implementation does nothing, making this method optional for
    /// actors that don't need special cleanup logic.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use lit_bit_core::actor::{Actor, ActorError};
    /// # struct MyActor { connection: Option<()> }
    /// impl Actor for MyActor {
    ///     type Message = String;
    ///     type Future<'a> = core::future::Ready<()> where Self: 'a;
    ///
    ///     fn handle(&mut self, _msg: String) -> Self::Future<'_> {
    ///         core::future::ready(())
    ///     }
    ///
    ///     fn on_cleanup(&mut self) -> Result<(), ActorError> {
    ///         // Close connection before restart
    ///         if let Some(_conn) = self.connection.take() {
    ///             // Clean up connection
    ///         }
    ///         Ok(())
    ///     }
    /// }
    #[allow(clippy::result_large_err)] // ActorError provides detailed failure information
    fn on_cleanup(&mut self) -> Result<(), ActorError> {
        // Default: no cleanup needed
        Ok(())
    }
}

/// Ergonomic async trait for use when heap allocation is available.
///
/// This trait provides a more ergonomic API using `async fn` syntax when the `std` or `alloc`
/// features are enabled. It automatically boxes futures to provide a uniform interface.
///
/// ## When to Use
///
/// - Use `AsyncActor` when you have `std` or `alloc` available and prefer ergonomic syntax
/// - Use `Actor` for `no_std` environments or when you need zero-cost abstractions
///
/// ## Automatic Implementation
///
/// Any type implementing `AsyncActor` automatically implements `Actor` via a blanket impl.
///
/// ## Examples
///
/// ```rust,no_run
/// # #[cfg(any(feature = "std", feature = "alloc"))]
/// # {
/// use lit_bit_core::actor::AsyncActor;
/// use futures::future::BoxFuture;
///
/// struct HttpActor {
///     // client: HttpClient,
/// }
///
/// struct HttpRequest {
///     url: String,
/// }
///
/// impl AsyncActor for HttpActor {
///     type Message = HttpRequest;
///
///     fn handle(&mut self, msg: HttpRequest) -> BoxFuture<'_, ()> {
///         Box::pin(async move {
///             // let response = self.client.get(&msg.url).await;
///             // Process response...
///         })
///     }
/// }
/// # }
/// ```
#[cfg(any(feature = "std", feature = "alloc"))]
pub trait AsyncActor: Send {
    /// The message type this actor handles
    type Message: Send + 'static;

    /// Handle a single message asynchronously using ergonomic async fn syntax.
    ///
    /// Note: This method returns a boxed future for ergonomic use when heap allocation
    /// is available. The actual implementation should use async fn syntax when possible.
    #[must_use]
    fn handle(&mut self, msg: Self::Message) -> futures::future::BoxFuture<'_, ()>;

    /// Called when the actor starts. Default: Ok(())
    ///
    /// # Errors
    /// Returns `Err(ActorError)` if actor startup fails.
    fn on_start(&mut self) -> Result<(), ActorError> {
        Ok(())
    }

    /// Called when the actor stops. Default: Ok(())
    ///
    /// # Errors
    /// Returns `Err(ActorError)` if actor shutdown fails.
    fn on_stop(self) -> Result<(), ActorError>
    where
        Self: Sized,
    {
        Ok(())
    }

    /// Called if the actor panics. Default: `RestartStrategy::OneForOne`
    fn on_panic(&self, _info: &PanicInfo) -> RestartStrategy {
        RestartStrategy::OneForOne
    }

    /// Called before restart to reset state.
    ///
    /// This hook allows actors to clean up state or perform initialization
    /// when restarted by a supervisor. Called after actor creation but before
    /// the first message is processed.
    ///
    /// # Returns
    /// `Ok(())` if restart preparation succeeds, `Err(ActorError)` if it fails
    fn on_restart(&mut self) -> Result<(), ActorError> {
        // Default: no special restart logic
        Ok(())
    }
}

/// Blanket implementation of Actor for any `AsyncActor` when heap allocation is available.
///
/// This allows `AsyncActor` implementations to be used anywhere Actor is expected,
/// providing seamless interoperability between the ergonomic and zero-cost APIs.
#[cfg(any(feature = "std", feature = "alloc"))]
impl<T> Actor for T
where
    T: AsyncActor,
{
    type Message = T::Message;
    type Future<'a>
        = futures::future::BoxFuture<'a, ()>
    where
        Self: 'a;

    fn handle(&mut self, msg: Self::Message) -> Self::Future<'_> {
        AsyncActor::handle(self, msg)
    }

    fn on_start(&mut self) -> Result<(), ActorError> {
        AsyncActor::on_start(self)
    }

    fn on_stop(self) -> Result<(), ActorError>
    where
        Self: Sized,
    {
        AsyncActor::on_stop(self)
    }

    fn on_panic(&self, info: &PanicInfo) -> RestartStrategy {
        AsyncActor::on_panic(self, info)
    }

    fn on_restart(&mut self) -> Result<(), ActorError> {
        AsyncActor::on_restart(self)
    }
}

// Conditional mailbox type aliases (Task 2.1)
#[cfg(not(feature = "async-tokio"))]
pub type Inbox<T, const N: usize> = heapless::spsc::Consumer<'static, T, N>;
#[cfg(not(feature = "async-tokio"))]
pub type Outbox<T, const N: usize> = heapless::spsc::Producer<'static, T, N>;

#[cfg(feature = "async-tokio")]
pub type Inbox<T> = tokio::sync::mpsc::Receiver<T>;
#[cfg(feature = "async-tokio")]
pub type Outbox<T> = tokio::sync::mpsc::Sender<T>;

// Platform-specific mailbox creation functions (Tasks 2.2-2.3)

/// Creates a static mailbox with safe initialization.
///
/// This macro creates a statically allocated SPSC queue using `StaticCell` and returns
/// the producer and consumer endpoints. It handles initialization safely without any
/// unsafe code and ensures the queue can only be split once.
///
/// # When to Use
///
/// Use this macro when you need:
/// - A simple, zero-allocation mailbox for actor communication
/// - Static allocation with automatic initialization
/// - No manual management of static cells
///
/// This is the recommended approach for most actor implementations.
///
/// # Arguments
///
/// * `$name` - Identifier for the static queue (for debugging/placement control)
/// * `$msg_type` - The message type for the queue
/// * `$capacity` - The queue capacity (const expression)
///
/// # Examples
///
/// ## Basic Actor Communication
/// ```rust,no_run
/// use lit_bit_core::{static_mailbox, Actor};
///
/// // Define message type
/// enum SensorMessage {
///     ReadTemperature,
///     SetThreshold(f32),
/// }
///
/// // Create actor with static mailbox
/// struct SensorActor;
/// impl Actor for SensorActor {
///     type Message = SensorMessage;
///     type Future<'a> = core::future::Ready<()> where Self: 'a;
///
///     fn handle(&mut self, msg: Self::Message) -> Self::Future<'_> {
///         core::future::ready(())
///     }
/// }
///
/// // Create a mailbox with capacity 16
/// let (producer, consumer) = static_mailbox!(SENSOR_MAILBOX: SensorMessage, 16);
///
/// // Send messages
/// producer.enqueue(SensorMessage::ReadTemperature).unwrap();
/// ```
///
/// ## Memory Section Placement
/// ```rust,no_run
/// // Place mailbox in specific memory section (e.g., fast SRAM)
/// let (tx, rx) = static_mailbox!(
///     #[link_section = ".sram2"]
///     FAST_MAILBOX: MyMessage, 32
/// );
/// ```
///
/// ## Multiple Independent Mailboxes
/// ```rust,no_run
/// // Create separate mailboxes for different message types
/// let (cmd_tx, cmd_rx) = static_mailbox!(COMMAND_MAILBOX: CommandMsg, 8);
/// let (evt_tx, evt_rx) = static_mailbox!(EVENT_MAILBOX: EventMsg, 32);
///
/// // Each mailbox operates independently
/// cmd_tx.enqueue(CommandMsg::Start).unwrap();
/// evt_tx.enqueue(EventMsg::Started).unwrap();
/// ```
///
/// # Panics
///
/// Panics if called more than once for the same static queue (prevents double-split).
#[macro_export]
macro_rules! static_mailbox {
    ($(#[$attr:meta])* $name:ident: $msg_type:ty, $capacity:expr) => {{
        $(#[$attr])*
        static $name: ::static_cell::StaticCell<::heapless::spsc::Queue<$msg_type, $capacity>> = ::static_cell::StaticCell::new();

        // Initialize the queue and get a 'static reference
        let queue: &'static mut ::heapless::spsc::Queue<$msg_type, $capacity> =
            $name.init(::heapless::spsc::Queue::new());

        // Split the queue into producer and consumer
        queue.split()
    }};

    // Variant without attributes
    ($name:ident: $msg_type:ty, $capacity:expr) => {{
        static $name: ::static_cell::StaticCell<::heapless::spsc::Queue<$msg_type, $capacity>> = ::static_cell::StaticCell::new();

        // Initialize the queue and get a 'static reference
        let queue: &'static mut ::heapless::spsc::Queue<$msg_type, $capacity> =
            $name.init(::heapless::spsc::Queue::new());

        // Split the queue into producer and consumer
        queue.split()
    }};
}

/// Creates a mailbox from a statically allocated queue using `StaticCell`.
///
/// This function provides a lower-level API for creating mailboxes from static memory,
/// giving you more control over the static allocation and initialization. It uses
/// `StaticCell` to ensure safe one-time initialization without any unsafe code.
///
/// # When to Use
///
/// Use this function when you need:
/// - Manual control over static cell creation and lifetime
/// - Custom initialization logic for the queue
/// - Integration with existing static storage patterns
/// - Sharing a single static cell between multiple components
///
/// For simpler cases, prefer the `static_mailbox!` macro.
///
/// # Arguments
///
/// * `cell` - A `StaticCell` containing an uninitialized heapless queue
///
/// # Examples
///
/// ## Basic Usage
/// ```rust,no_run
/// use heapless::spsc::Queue;
/// use static_cell::StaticCell;
/// use lit_bit_core::actor::create_mailbox;
///
/// // Define static storage
/// static QUEUE_CELL: StaticCell<Queue<u32, 16>> = StaticCell::new();
///
/// // Create mailbox when needed
/// let (outbox, inbox) = create_mailbox(&QUEUE_CELL);
/// ```
///
/// ## Shared Static Cell
/// ```rust,no_run
/// use heapless::spsc::Queue;
/// use static_cell::StaticCell;
/// use lit_bit_core::actor::create_mailbox;
///
/// // Module-level static cell
/// pub(crate) static SHARED_QUEUE: StaticCell<Queue<Event, 32>> = StaticCell::new();
///
/// // Function to initialize subsystem
/// fn init_subsystem() {
///     // Initialize the queue once
///     let (tx, rx) = create_mailbox(&SHARED_QUEUE);
///     // Use tx/rx...
/// }
/// ```
///
/// ## Custom Placement with Attributes
/// ```rust,no_run
/// use heapless::spsc::Queue;
/// use static_cell::StaticCell;
/// use lit_bit_core::actor::create_mailbox;
///
/// // Place queue in specific memory section
/// #[link_section = ".dma_memory"]
/// static DMA_QUEUE: StaticCell<Queue<u8, 64>> = StaticCell::new();
///
/// fn setup_dma() {
///     let (producer, consumer) = create_mailbox(&DMA_QUEUE);
///     // Configure DMA with producer/consumer...
/// }
/// ```
///
/// # Platform-Specific Behavior
///
/// - In `no_std` environments, uses `heapless::Queue` for zero-allocation storage
/// - In `std` with `async-tokio` feature, uses `tokio::sync::mpsc::channel`
/// - Memory overhead is determined by message type size and capacity
/// - Queue capacity must be known at compile time in `no_std` mode
#[cfg(not(feature = "async-tokio"))]
#[must_use]
pub fn create_mailbox<T, const N: usize>(
    cell: &'static StaticCell<heapless::spsc::Queue<T, N>>,
) -> (Outbox<T, N>, Inbox<T, N>) {
    let queue = cell.init(heapless::spsc::Queue::new());
    queue.split()
}

#[cfg(feature = "async-tokio")]
pub fn create_mailbox<T>(capacity: usize) -> (Outbox<T>, Inbox<T>) {
    tokio::sync::mpsc::channel(capacity)
}

#[cfg(not(feature = "async-tokio"))]
#[macro_export]
macro_rules! define_static_mailbox {
    ($name:ident, $type:ty, $size:expr) => {
        static $name: ::static_cell::StaticCell<::heapless::spsc::Queue<$type, $size>> =
            ::static_cell::StaticCell::new();
    };
}

/// Yield mechanism for `no_std` environments without Embassy.
///
/// This provides a default yield implementation that allows the executor to schedule
/// other tasks when the message queue is empty. The implementation returns `Poll::Pending`
/// once before completing, which gives the executor an opportunity to run other tasks.
///
/// ## Future Improvement
///
/// TODO: Replace with `core::task::yield_now()` when it stabilizes (currently behind
/// `#![feature(async_yield)]` in nightly). This will simplify the implementation
/// and provide better integration with the standard library.
///
/// ## Customization for Different Executors
///
/// Different async executors may require different yield mechanisms:
///
/// - **Embassy**: Uses `embassy_futures::yield_now()` (handled separately)
/// - **RTIC**: May use `rtic_monotonics::yield_now()` or similar
/// - **Custom executors**: May need executor-specific yield functions
///
/// If you're using a different executor, you may need to replace this function
/// with your executor's specific yield mechanism. This can be done by:
///
/// 1. Defining your own yield function with the same signature
/// 2. Using conditional compilation to select the appropriate implementation
/// 3. Or by configuring your executor to handle this default yield appropriately
///
/// ## Implementation Notes
///
/// This implementation creates a future that:
/// 1. Returns `Poll::Pending` on first poll (yielding control)
/// 2. Wakes itself to be polled again
/// 3. Returns `Poll::Ready(())` on second poll (completing)
///
/// This ensures the message loop doesn't busy-wait when no messages are available,
/// while still allowing rapid message processing when messages are present.
#[cfg(all(not(feature = "async-tokio"), not(feature = "embassy")))]
async fn yield_control() {
    use core::future::Future;
    use core::pin::Pin;
    use core::task::{Context, Poll};

    /// A future that yields control once before completing
    struct YieldOnce {
        yielded: bool,
    }

    impl Future for YieldOnce {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.yielded {
                Poll::Ready(())
            } else {
                self.yielded = true;
                cx.waker().wake_by_ref(); // Schedule this task to be polled again
                Poll::Pending
            }
        }
    }

    YieldOnce { yielded: false }.await;
}

/// Message processing loop implementation (Task 3.1)
/// Runs an actor's message processing loop.
///
/// # Errors
/// Returns `ActorError` if actor startup, shutdown, or message processing fails.
#[allow(unreachable_code)] // no_std path has infinite loop, cleanup only reachable on std
#[cfg(all(not(feature = "async-tokio"), not(feature = "async-embassy")))]
pub async fn actor_task<A: Actor, const N: usize>(
    mut actor: A,
    mut inbox: Inbox<A::Message, N>,
) -> Result<(), ActorError> {
    // Startup hook
    let startup_result = actor.on_start();
    #[cfg(feature = "debug-log")]
    if let Err(ref e) = startup_result {
        log::error!("Actor startup failed: {e:?}");
    }
    startup_result?;

    // Main processing loop (Ector pattern)
    loop {
        let msg = loop {
            if let Some(msg) = inbox.dequeue() {
                break msg;
            }
            // Yield and continue (Embassy style)
            #[cfg(feature = "embassy")]
            embassy_futures::yield_now().await;
            #[cfg(not(feature = "embassy"))]
            {
                // For no_std without embassy, yield control to allow other tasks to run.
                // This uses a configurable yield function that can be customized per executor.
                yield_control().await;
            }
        };
        actor.handle(msg).await;
    }

    // Cleanup hook (unreachable in no_std)
    #[allow(unreachable_code)]
    {
        let stop_result = actor.on_stop();
        #[cfg(feature = "debug-log")]
        if let Err(ref e) = stop_result {
            log::error!("Actor shutdown failed: {e:?}");
        }
        stop_result?;
        Ok(())
    }
}

/// Runs an actor's message processing loop (Embassy version).
///
/// This function implements the Embassy-specific actor task that integrates with
/// Embassy's channel system and cooperative scheduler. It follows Embassy 0.6
/// best practices for message processing and task lifecycle management.
///
/// ## Embassy Integration
///
/// - Uses `embassy_sync::channel::Receiver` for message reception
/// - Integrates with Embassy's cooperative task scheduler
/// - Provides deterministic message processing (one at a time)
/// - Handles actor lifecycle hooks (startup/shutdown)
///
/// ## Error Handling
///
/// In embedded environments, error handling is typically simpler than in
/// desktop applications. This function logs errors when debug logging is
/// available but doesn't attempt complex recovery strategies.
///
/// # Arguments
///
/// * `actor` - The actor instance to run
/// * `receiver` - Embassy channel receiver for incoming messages
///
/// # Errors
/// Returns `ActorError` if actor startup or shutdown fails.
/// Message processing errors are handled internally.
#[cfg(feature = "async-embassy")]
pub async fn actor_task_embassy<A, const N: usize>(
    mut actor: A,
    receiver: embassy_sync::channel::Receiver<
        'static,
        embassy_sync::blocking_mutex::raw::NoopRawMutex,
        A::Message,
        N,
    >,
) -> Result<(), ActorError>
where
    A: Actor,
    A::Message: Send + 'static,
{
    // Startup hook
    let startup_result = actor.on_start();
    #[cfg(feature = "debug-log")]
    if let Err(ref e) = startup_result {
        log::error!("Actor startup failed: {e:?}");
    }
    startup_result?;

    // Main message processing loop
    // In Embassy, this loop will cooperatively yield when no messages are available
    loop {
        // Wait for next message - this will suspend the task if no messages available
        // Embassy's channel receiver integrates with the cooperative scheduler
        let msg = receiver.receive().await;

        // Process the message atomically (one at a time)
        // This ensures deterministic execution and prevents re-entrancy
        actor.handle(msg).await;
    }

    // Note: This cleanup code is unreachable in the infinite loop above,
    // but included for completeness. In embedded systems, actors typically
    // run forever until device reset.
    #[allow(unreachable_code)]
    {
        let stop_result = actor.on_stop();
        #[cfg(feature = "debug-log")]
        if let Err(ref e) = stop_result {
            log::error!("Actor shutdown failed: {e:?}");
        }
        stop_result?;
        Ok(())
    }
}

/// Runs an actor's message processing loop (std version).
///
/// # Errors
/// Returns `ActorError` if actor startup, shutdown, or message processing fails.
#[cfg(feature = "async-tokio")]
pub async fn actor_task<A>(mut actor: A, mut inbox: Inbox<A::Message>) -> Result<(), ActorError>
where
    A: Actor + Send + 'static,
    A::Message: Send + 'static,
{
    // Start the actor
    actor.on_start()?;

    // Process messages until the channel is closed
    while let Some(msg) = inbox.recv().await {
        let future = actor.handle(msg);
        future.await;
    }

    // Cleanup hook - call on_stop when the channel is closed
    let stop_result = actor.on_stop();
    #[cfg(feature = "debug-log")]
    if let Err(ref e) = stop_result {
        log::error!("Actor shutdown failed: {e:?}");
    }
    stop_result?;

    Ok(())
}

/// Runs a batch-aware actor's message processing loop (Embassy version).
///
/// This function implements batch processing for Embassy actors, following the research
/// document's recommendations for zero-allocation message batching. It drains available
/// messages from the channel and processes them in batches.
///
/// ## Batching Strategy
///
/// - Waits for at least one message (blocking)
/// - Drains all available messages up to `max_batch_size()`
/// - Processes the batch in a single `handle_batch()` call
/// - Yields control after each batch (cooperative scheduling)
///
/// ## Performance Benefits
///
/// - Fewer Embassy channel receive operations
/// - Better cache locality for related messages
/// - Reduced task switching overhead
/// - Higher throughput for high-frequency message scenarios
///
/// # Arguments
///
/// * `actor` - The batch actor instance to run
/// * `receiver` - Embassy channel receiver for incoming messages
///
/// # Errors
/// Returns `ActorError` if actor startup or shutdown fails.
#[cfg(feature = "async-embassy")]
pub async fn batch_actor_task_embassy<A, const N: usize>(
    mut actor: A,
    receiver: embassy_sync::channel::Receiver<
        'static,
        embassy_sync::blocking_mutex::raw::NoopRawMutex,
        A::Message,
        N,
    >,
) -> Result<(), ActorError>
where
    A: BatchActor,
    A::Message: Send + 'static,
{
    // Startup hook
    let startup_result = actor.on_start();
    #[cfg(feature = "debug-log")]
    if let Err(ref e) = startup_result {
        log::error!("Batch actor startup failed: {e:?}");
    }
    startup_result?;

    // Prepare a static buffer for batching messages
    // Using heapless for zero-allocation message collection
    let mut batch_buffer: heapless::Vec<A::Message, 64> = heapless::Vec::new();

    // Main batch processing loop
    loop {
        // Wait for at least one message
        let first_message = receiver.receive().await;
        batch_buffer.clear();
        batch_buffer.push(first_message).ok(); // Safe: buffer is empty

        // Drain additional messages up to batch limit
        let max_batch = actor.max_batch_size().min(64); // Constrained by buffer size
        while batch_buffer.len() < max_batch {
            match receiver.try_receive() {
                Ok(msg) => {
                    if batch_buffer.push(msg).is_err() {
                        break; // Buffer full
                    }
                }
                Err(_) => break, // No more messages available
            }
        }

        // Process the batch
        actor.handle_batch(&batch_buffer).await;

        // Yield control to maintain cooperative scheduling
        #[cfg(feature = "embassy")]
        embassy_futures::yield_now().await;
    }

    // Cleanup hook (unreachable in embedded)
    #[allow(unreachable_code)]
    {
        let stop_result = actor.on_stop();
        #[cfg(feature = "debug-log")]
        if let Err(ref e) = stop_result {
            log::error!("Batch actor shutdown failed: {e:?}");
        }
        stop_result?;
        Ok(())
    }
}

/// Runs a batch-aware actor's message processing loop (Tokio version).
///
/// This function implements batch processing for Tokio actors, using Tokio's channel
/// capabilities to efficiently drain pending messages and process them in batches.
///
/// ## Batching Strategy
///
/// - Uses `recv().await` for the first message (blocking)
/// - Uses `try_recv()` to drain additional messages without blocking
/// - Processes batches up to `max_batch_size()` messages
/// - Respects Tokio's cooperative scheduling budget
///
/// ## Performance Benefits
///
/// - Fewer Tokio channel operations
/// - Reduced task wake-up overhead
/// - Better throughput for high-frequency messaging
/// - Maintained fairness through batch size limits
///
/// # Arguments
///
/// * `actor` - The batch actor instance to run
/// * `inbox` - Tokio channel receiver for incoming messages
///
/// # Errors
/// Returns `ActorError` if actor startup or shutdown fails.
#[cfg(feature = "async-tokio")]
pub async fn batch_actor_task<A>(
    mut actor: A,
    mut inbox: Inbox<A::Message>,
) -> Result<(), ActorError>
where
    A: BatchActor + Send + 'static,
    A::Message: Send + 'static,
{
    // Start the actor
    actor.on_start()?;

    // Process messages in batches
    let mut batch = Vec::with_capacity(actor.max_batch_size());

    // Main batch processing loop - exit when channel closes
    while let Some(first_msg) = inbox.recv().await {
        // Start with the first message
        batch.clear();
        batch.push(first_msg);

        // Try to drain additional messages without blocking
        while batch.len() < actor.max_batch_size() {
            match inbox.try_recv() {
                Ok(msg) => batch.push(msg),
                Err(_) => break, // No more messages available right now
            }
        }

        // Process the batch
        let future = actor.handle_batch(&batch);
        future.await;
    }

    // Cleanup hook - call on_stop when the channel is closed
    let stop_result = actor.on_stop();
    #[cfg(feature = "debug-log")]
    if let Err(ref e) = stop_result {
        log::error!("Batch actor shutdown failed: {e:?}");
    }
    stop_result?;

    Ok(())
}

/// Runs a batch-aware actor's message processing loop (no_std version).
///
/// This function implements batch processing for no_std environments without Embassy,
/// using heapless SPSC queues for zero-allocation message batching.
///
/// ## Batching Strategy
///
/// - Polls for the first message with yielding
/// - Drains all available messages from the SPSC queue
/// - Processes batches up to `max_batch_size()` messages
/// - Uses configurable yield mechanism for executor compatibility
///
/// # Arguments
///
/// * `actor` - The batch actor instance to run
/// * `inbox` - Heapless SPSC consumer for incoming messages
///
/// # Errors
/// Returns `ActorError` if actor startup or shutdown fails.
#[cfg(all(not(feature = "async-tokio"), not(feature = "async-embassy")))]
pub async fn batch_actor_task<A: BatchActor, const N: usize>(
    mut actor: A,
    mut inbox: Inbox<A::Message, N>,
) -> Result<(), ActorError> {
    // Startup hook
    let startup_result = actor.on_start();
    #[cfg(feature = "debug-log")]
    if let Err(ref e) = startup_result {
        log::error!("Batch actor startup failed: {e:?}");
    }
    startup_result?;

    // Prepare a static buffer for batching messages
    let mut batch_buffer: heapless::Vec<A::Message, 64> = heapless::Vec::new();

    // Main batch processing loop
    loop {
        // Wait for at least one message
        let first_message = loop {
            if let Some(msg) = inbox.dequeue() {
                break msg;
            }
            // Yield and continue
            yield_control().await;
        };

        batch_buffer.clear();
        batch_buffer.push(first_message).ok(); // Safe: buffer is empty

        // Drain additional messages up to batch limit
        let max_batch = actor.max_batch_size().min(64); // Constrained by buffer size
        while batch_buffer.len() < max_batch {
            if let Some(msg) = inbox.dequeue() {
                if batch_buffer.push(msg).is_err() {
                    break; // Buffer full
                }
            } else {
                break; // No more messages available
            }
        }

        // Process the batch
        actor.handle_batch(&batch_buffer).await;

        // Yield control to allow other tasks to run
        yield_control().await;
    }

    // Cleanup hook (unreachable in no_std)
    #[allow(unreachable_code)]
    {
        let stop_result = actor.on_stop();
        #[cfg(feature = "debug-log")]
        if let Err(ref e) = stop_result {
            log::error!("Batch actor shutdown failed: {e:?}");
        }
        stop_result?;
        Ok(())
    }
}

pub mod address;
pub mod backpressure;
pub mod integration;
pub mod panic_handling;
pub mod spawn;
pub mod supervision; // Task 5.1: Supervision with Async // Task 5.4: Advanced Error Handling

// Re-export spawn functions for convenience
#[cfg(feature = "async-embassy")]
pub use spawn::spawn_counter_actor_embassy;
#[cfg(all(feature = "async-tokio", not(feature = "async-embassy")))]
pub use spawn::{
    spawn_actor_tokio, spawn_batch_actor_tokio, spawn_supervised_actor_tokio,
    spawn_supervised_batch_actor_tokio,
};

// Re-export supervision types for convenience (Task 5.1 & 5.4)
pub use supervision::{SupervisorActor, SupervisorError, SupervisorTimer};

// Re-export panic handling utilities for convenience (Task 5.4)
pub use panic_handling::create_controlled_failure;

#[cfg(feature = "async-tokio")]
pub use panic_handling::{
    capture_panic_info, capture_panic_info_from_payload, capture_panic_info_from_payload_with_id,
    capture_panic_info_with_id,
};

#[cfg(feature = "async-embassy")]
pub use panic_handling::{simulate_panic_for_testing, simulate_panic_with_id};

/// Escalation policy for hierarchical supervision.
///
/// Defines how a supervisor should handle failures that exceed restart intensity limits.
/// This enables multi-level supervision trees where failures can be escalated upward
/// for higher-level intervention.
#[derive(Debug, Clone)]
pub enum EscalationPolicy {
    /// Supervisor terminates itself on escalation (simple meltdown)
    /// Use when the supervisor should fail-fast and let its parent handle recovery
    TerminateSelf,
    /// Send escalation message to parent supervisor (controlled escalation)
    /// Use when you want explicit parent notification with custom handling
    NotifyParent,
    /// Apply custom escalation logic via trait method
    /// Use when you need application-specific escalation behavior
    Custom,
}

impl Default for EscalationPolicy {
    fn default() -> Self {
        Self::TerminateSelf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test actor for unit testing
    struct TestActor {
        counter: u32,
    }

    impl TestActor {
        fn new() -> Self {
            Self { counter: 0 }
        }
    }

    impl Actor for TestActor {
        type Message = u32;
        type Future<'a>
            = core::future::Ready<()>
        where
            Self: 'a;

        fn handle(&mut self, msg: Self::Message) -> Self::Future<'_> {
            self.counter += msg;
            core::future::ready(())
        }
    }

    #[test]
    fn actor_trait_compiles() {
        let mut actor = TestActor::new();
        assert_eq!(actor.counter, 0);

        // Test lifecycle hooks
        assert!(actor.on_start().is_ok());
        assert!(actor.on_stop().is_ok());
    }

    #[cfg(all(not(feature = "std"), not(feature = "embassy")))]
    #[test]
    fn yield_control_compiles() {
        // Test that our yield mechanism compiles and can be used in async contexts
        // This is a compile-time test to ensure the yield function is properly defined
        let _future = yield_control();
        // Note: We can't easily test the actual yielding behavior in a unit test
        // without a full async runtime, but we can verify it compiles correctly
    }

    #[cfg(not(feature = "std"))]
    #[test]
    fn static_mailbox_macro_works() {
        // Test that our static_mailbox macro works correctly
        let (mut producer, mut consumer) = crate::static_mailbox!(TEST_MAILBOX: u32, 4);

        // Test basic functionality
        assert!(producer.enqueue(42).is_ok());
        assert_eq!(consumer.dequeue(), Some(42));
        assert_eq!(consumer.dequeue(), None);
    }

    #[cfg(not(feature = "std"))]
    #[test]
    fn static_mailbox_multiple_instances() {
        // Test that multiple static_mailbox! invocations don't conflict
        let (mut producer1, mut consumer1) = crate::static_mailbox!(MAILBOX_ONE: u32, 4);
        let (mut producer2, mut consumer2) = crate::static_mailbox!(MAILBOX_TWO: i32, 8);

        // Test both mailboxes work independently
        assert!(producer1.enqueue(123).is_ok());
        assert!(producer2.enqueue(456).is_ok());

        assert_eq!(consumer1.dequeue(), Some(123));
        assert_eq!(consumer2.dequeue(), Some(456));

        // Verify they're independent
        assert_eq!(consumer1.dequeue(), None);
        assert_eq!(consumer2.dequeue(), None);
    }

    #[test]
    fn static_mailbox_zero_allocation() {
        // Create a static mailbox with capacity 16
        let (mut producer, mut consumer) = static_mailbox!(ZERO_ALLOC_TEST: u32, 16);

        // Send and receive messages without any heap allocation
        assert!(producer.enqueue(42).is_ok());
        assert!(producer.enqueue(43).is_ok());

        assert_eq!(consumer.dequeue(), Some(42));
        assert_eq!(consumer.dequeue(), Some(43));
        assert_eq!(consumer.dequeue(), None);

        // Verify we can reuse the queue
        assert!(producer.enqueue(44).is_ok());
        assert_eq!(consumer.dequeue(), Some(44));
    }

    #[test]
    fn static_mailbox_capacity_limits() {
        // Create a small mailbox to test capacity limits
        // Note: A queue with size parameter 2 can only hold 1 element
        let (mut producer, mut consumer) = static_mailbox!(CAPACITY_TEST: u32, 2);

        // Fill the queue (can only hold 1 element)
        assert!(producer.enqueue(1).is_ok());
        // Queue is now full (N-1 capacity)
        assert!(producer.enqueue(2).is_err());

        // After dequeuing, we can enqueue again
        assert_eq!(consumer.dequeue(), Some(1));
        assert!(producer.enqueue(2).is_ok());
    }

    #[test]
    fn static_mailbox_multiple_independent() {
        // Create two independent mailboxes
        let (mut p1, mut c1) = static_mailbox!(MULTI_TEST_1: u32, 4);
        let (mut p2, mut c2) = static_mailbox!(MULTI_TEST_2: u32, 4);

        // Verify they operate independently
        assert!(p1.enqueue(1).is_ok());
        assert!(p2.enqueue(100).is_ok());

        assert_eq!(c1.dequeue(), Some(1));
        assert_eq!(c2.dequeue(), Some(100));

        // Each queue maintains its own state
        assert_eq!(c1.dequeue(), None);
        assert_eq!(c2.dequeue(), None);
    }
}
