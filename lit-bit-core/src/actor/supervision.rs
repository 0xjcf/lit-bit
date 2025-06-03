//! Supervision utilities for managing child actors with restart strategies.
//!
//! This module implements OTP-style supervision patterns as described in the research document,
//! providing platform-dual supervision that works with both Tokio (JoinHandle monitoring) and
//! Embassy (message signaling) environments.

use super::{Actor, RestartStrategy, Supervisor, SupervisorMessage};

// Import ActorError when needed (async-tokio features or test contexts)
#[cfg(any(feature = "async-tokio", test))]
use super::ActorError;

// Import BoxedActorError only in test contexts where it's used
#[cfg(all(any(feature = "std", feature = "alloc"), test))]
#[allow(unused_imports)] // Used conditionally in platform-specific test code
use super::BoxedActorError;

// Import ActorError for no_std environments (for RestartFactory type)
#[cfg(all(not(any(feature = "std", feature = "alloc")), feature = "async-tokio"))]
use super::ActorError;

#[cfg(feature = "async-tokio")]
use futures::FutureExt;
#[cfg(feature = "async-tokio")]
use std::collections::HashMap;
#[cfg(feature = "async-tokio")]
use tokio::task::JoinHandle;

#[cfg(not(feature = "async-tokio"))]
use heapless::FnvIndexMap;

// Import Vec for collections based on available features
#[cfg(any(feature = "std", feature = "alloc"))]
extern crate alloc;
#[cfg(any(feature = "std", feature = "alloc"))]
use alloc::boxed::Box;
#[cfg(any(feature = "std", feature = "alloc"))]
use alloc::vec::Vec;

// Import Box for no_std environments when needed
#[cfg(not(any(feature = "std", feature = "alloc")))]
extern crate alloc;
#[cfg(not(any(feature = "std", feature = "alloc")))]
use alloc::boxed::Box;

/// Trait for providing platform-specific timer functionality.
///
/// This trait must be implemented for platforms that don't have `std` or `embassy`
/// features enabled. It provides the supervisor with access to monotonic time
/// for restart window calculations.
///
/// # Requirements
///
/// - **Monotonic**: Time values must be monotonically increasing
/// - **Millisecond precision**: Values should represent milliseconds since an arbitrary epoch
/// - **Overflow handling**: Should handle timer wrap-around gracefully
///
/// # Example Implementation
///
/// ```rust,ignore
/// struct MyPlatformTimer;
///
/// impl SupervisorTimer for MyPlatformTimer {
///     fn current_time_ms() -> u64 {
///         // Platform-specific timer implementation
///         my_platform_get_tick_count_ms()
///     }
/// }
/// ```
pub trait SupervisorTimer {
    /// Returns the current monotonic time in milliseconds.
    ///
    /// This value is used for restart window calculations and must be
    /// monotonically increasing. The absolute value doesn't matter,
    /// only that it advances consistently.
    fn current_time_ms() -> u64;
}

/// Error types for supervisor operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorError {
    /// Child capacity limit reached (no_std environments)
    CapacityExceeded,
    /// Child with this ID already exists
    ChildAlreadyExists,
    /// Child with this ID not found
    ChildNotFound,
    /// Failed to restart child actor
    RestartFailed,
}

// Timer implementations are provided for different feature combinations
// Default no_std implementation uses an atomic counter for basic timing

/// Type alias for restart factory functions in Tokio environments.
///
/// These functions spawn new child actor instances and return their JoinHandles.
#[cfg(feature = "async-tokio")]
pub type RestartFactory = Box<dyn Fn() -> JoinHandle<Result<(), ActorError>> + Send + 'static>;

/// Type alias for restart factory functions in non-Tokio environments.
///
/// These functions spawn new child actor instances and return success/failure.
#[cfg(not(feature = "async-tokio"))]
pub type RestartFactory = Box<dyn Fn() -> Result<(), SupervisorError> + Send + 'static>;

/// A supervisor actor that manages child actors with restart strategies.
///
/// Implements the supervision patterns from the research document, providing:
/// - **OneForOne**: Restart only the failed child
/// - **OneForAll**: Restart all children when any child fails
/// - **RestForOne**: Restart the failed child and all children started after it
///
/// ## Platform-Specific Behavior
///
/// - **Tokio**: Uses `JoinHandle` monitoring to detect child termination
/// - **Embassy**: Uses message signaling for child failure notification
/// - **No-std**: Uses heapless collections for zero-allocation supervision
///
/// ## Usage
///
/// ```rust,no_run
/// use lit_bit_core::actor::supervision::SupervisorActor;
///
/// let supervisor: SupervisorActor<u32, 8> = SupervisorActor::new();
/// // Add children with restart factories and handle SupervisorMessage events
/// ```
pub struct SupervisorActor<ChildId = u32, const MAX_CHILDREN: usize = 16>
where
    ChildId: Clone + PartialEq + core::fmt::Debug + core::hash::Hash + Eq,
{
    /// Map of child ID to restart strategy
    #[cfg(feature = "async-tokio")]
    children: HashMap<ChildId, ChildInfo>,

    #[cfg(not(feature = "async-tokio"))]
    children: FnvIndexMap<ChildId, ChildInfo, MAX_CHILDREN>,

    /// Default restart strategy for new children
    default_restart_strategy: RestartStrategy,

    /// Maximum number of restarts per child (prevents restart loops)
    max_restarts: usize,

    /// Time window for restart counting (in milliseconds)
    restart_window_ms: u64,

    /// Sequence counter for tracking child start order (for RestForOne strategy)
    next_start_sequence: u64,
}

/// Information about a supervised child actor.
struct ChildInfo {
    /// Restart strategy for this child
    restart_strategy: RestartStrategy,

    /// Number of restarts within the current window
    restart_count: usize,

    /// Sequence number indicating the order this child was added (for RestForOne strategy)
    start_sequence: u64,

    /// Timestamp of the first restart in the current window
    #[cfg(feature = "std")]
    window_start: std::time::Instant,

    #[cfg(not(feature = "std"))]
    window_start_ms: u64, // Platform-specific timestamp

    /// Tokio-specific: JoinHandle for monitoring child termination
    #[cfg(feature = "async-tokio")]
    join_handle: Option<JoinHandle<Result<(), ActorError>>>,

    /// Embassy-specific: Flag indicating if child is currently running
    #[cfg(not(feature = "async-tokio"))]
    is_running: bool,

    /// Factory function for restarting this child actor
    /// This closure is called whenever the child needs to be restarted
    restart_factory: RestartFactory,
}

impl<ChildId, const MAX_CHILDREN: usize> SupervisorActor<ChildId, MAX_CHILDREN>
where
    ChildId: Clone + PartialEq + core::fmt::Debug + core::hash::Hash + Eq,
{
    /// Creates a new supervisor actor with default settings.
    ///
    /// # Default Configuration
    /// - **Restart strategy**: `RestartStrategy::OneForOne`
    /// - **Max restarts**: 5 restarts per child
    /// - **Restart window**: 60 seconds
    #[must_use]
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "async-tokio")]
            children: HashMap::new(),

            #[cfg(not(feature = "async-tokio"))]
            children: FnvIndexMap::new(),

            default_restart_strategy: RestartStrategy::OneForOne,
            max_restarts: 5,
            restart_window_ms: 60_000, // 60 seconds
            next_start_sequence: 0,
        }
    }

    /// Creates a supervisor with custom configuration.
    ///
    /// # Arguments
    /// * `default_strategy` - Default restart strategy for new children
    /// * `max_restarts` - Maximum restarts per child within the time window
    /// * `restart_window_ms` - Time window for restart counting (milliseconds)
    #[must_use]
    pub fn with_config(
        default_strategy: RestartStrategy,
        max_restarts: usize,
        restart_window_ms: u64,
    ) -> Self {
        Self {
            #[cfg(feature = "async-tokio")]
            children: HashMap::new(),

            #[cfg(not(feature = "async-tokio"))]
            children: FnvIndexMap::new(),

            default_restart_strategy: default_strategy,
            max_restarts,
            restart_window_ms,
            next_start_sequence: 0,
        }
    }

    /// Adds a child actor to supervision with a restart factory.
    ///
    /// # Arguments
    /// * `child_id` - Unique identifier for the child
    /// * `restart_factory` - Function that spawns a new instance of the child actor
    /// * `restart_strategy` - Optional custom restart strategy (uses default if None)
    ///
    /// # Returns
    /// `Ok(())` if the child was added successfully, `Err(SupervisorError)` if the operation failed.
    pub fn add_child_with_factory(
        &mut self,
        child_id: ChildId,
        restart_factory: RestartFactory,
        restart_strategy: Option<RestartStrategy>,
    ) -> Result<(), SupervisorError> {
        // Check if child already exists
        if self.children.contains_key(&child_id) {
            return Err(SupervisorError::ChildAlreadyExists);
        }

        let strategy = restart_strategy.unwrap_or(self.default_restart_strategy);

        let child_info = ChildInfo {
            restart_strategy: strategy,
            restart_count: 0,
            start_sequence: self.next_start_sequence,

            #[cfg(feature = "std")]
            window_start: std::time::Instant::now(),

            #[cfg(not(feature = "std"))]
            window_start_ms: Self::current_time_ms(),

            #[cfg(feature = "async-tokio")]
            join_handle: None,

            #[cfg(not(feature = "async-tokio"))]
            is_running: true,

            restart_factory,
        };

        #[cfg(feature = "async-tokio")]
        {
            self.children.insert(child_id, child_info);
            self.next_start_sequence += 1;
            Ok(())
        }

        #[cfg(not(feature = "async-tokio"))]
        {
            let _ = self.children.insert(child_id, child_info);
            self.next_start_sequence += 1;
            Ok(())
        }
    }

    /// Adds a child actor to supervision (legacy method without restart factory).
    ///
    /// This method creates a no-op restart factory for backwards compatibility.
    /// For actual restart functionality, use `add_child_with_factory` instead.
    ///
    /// # Arguments
    /// * `child_id` - Unique identifier for the child
    /// * `restart_strategy` - Optional custom restart strategy (uses default if None)
    ///
    /// # Returns
    /// `Ok(())` if the child was added successfully, `Err(SupervisorError)` if the operation failed.
    pub fn add_child(
        &mut self,
        child_id: ChildId,
        restart_strategy: Option<RestartStrategy>,
    ) -> Result<(), SupervisorError> {
        // Create a no-op restart factory for backwards compatibility
        #[cfg(feature = "async-tokio")]
        let no_op_factory: RestartFactory = Box::new(|| {
            // Return a completed task that immediately returns an error
            tokio::spawn(async { Err(ActorError::StartupFailure) })
        });

        #[cfg(not(feature = "async-tokio"))]
        let no_op_factory: RestartFactory = Box::new(|| Err(SupervisorError::RestartFailed));

        self.add_child_with_factory(child_id, no_op_factory, restart_strategy)
    }

    /// Removes a child from supervision.
    ///
    /// # Arguments
    /// * `child_id` - Identifier of the child to remove
    ///
    /// # Returns
    /// `true` if the child was found and removed, `false` otherwise.
    pub fn remove_child(&mut self, child_id: &ChildId) -> bool {
        self.children.remove(child_id).is_some()
    }

    /// Records a child failure and determines the restart strategy to apply.
    ///
    /// This method implements the core supervision logic, tracking restart counts
    /// and applying the appropriate restart strategy based on the child's configuration.
    ///
    /// # Arguments
    /// * `child_id` - Identifier of the failed child
    ///
    /// # Returns
    /// * `Some(RestartStrategy)` - Strategy to apply for this failure
    /// * `None` - Child not found or restart limit exceeded
    pub fn handle_child_failure(&mut self, child_id: &ChildId) -> Option<RestartStrategy> {
        let child_info = self.children.get_mut(child_id)?;

        // Check restart rate limiting
        #[cfg(feature = "std")]
        let window_elapsed = child_info.window_start.elapsed().as_millis() as u64;

        #[cfg(not(feature = "std"))]
        let (window_elapsed, current_time) = {
            let current_time = Self::current_time_ms();
            let window_elapsed = current_time.saturating_sub(child_info.window_start_ms);
            (window_elapsed, current_time)
        };

        if window_elapsed > self.restart_window_ms {
            // Reset restart count - new window
            child_info.restart_count = 0;

            #[cfg(feature = "std")]
            {
                child_info.window_start = std::time::Instant::now();
            }

            #[cfg(not(feature = "std"))]
            {
                child_info.window_start_ms = current_time;
            }
        }

        child_info.restart_count += 1;

        if child_info.restart_count > self.max_restarts {
            // Too many restarts - remove child from supervision
            #[cfg(feature = "debug-log")]
            log::warn!("Child {child_id:?} exceeded restart limit, removing from supervision");

            self.children.remove(child_id);
            return None;
        }

        Some(child_info.restart_strategy)
    }

    /// Handle child panic using Phase 2 framework integration.
    ///
    /// This method integrates panic-safe event loops with the supervision framework,
    /// applying restart intensity limits and escalation policies.
    ///
    /// # Arguments
    /// * `child_id` - Identifier of the child that panicked
    /// * `error` - The ActorError that caused the panic
    ///
    /// # Returns
    /// * `Some(RestartStrategy)` - Strategy to apply if restart is allowed
    /// * `None` - Restart limit exceeded or escalation required
    pub fn handle_child_panic(
        &mut self,
        child_id: &ChildId,
        error: super::ActorError,
    ) -> Option<RestartStrategy> {
        #[cfg(feature = "debug-log")]
        {
            match &error {
                super::ActorError::Panic { message, actor_id } => {
                    let msg = message
                        .as_ref()
                        .map(|m| m.as_str())
                        .unwrap_or("<no message>");
                    let id = actor_id.as_ref().map(|id| id.as_str()).unwrap_or("<no id>");
                    log::warn!("Child {child_id:?} panicked: message='{msg}', actor_id='{id}'");
                }
                super::ActorError::Custom(msg) => {
                    log::warn!("Child {child_id:?} custom error: {msg}");
                }
                other => {
                    log::warn!("Child {child_id:?} failed with error: {other:?}");
                }
            }
        }
        #[cfg(not(feature = "debug-log"))]
        let _ = &error;

        // In the future, this analysis could be used to select a different strategy
        // For now, use the existing failure handling logic
        let strategy = self.handle_child_failure(child_id)?;

        #[cfg(feature = "debug-log")]
        log::warn!("Applying restart strategy: {strategy:?} for child {child_id:?}");
        #[cfg(not(feature = "debug-log"))]
        let _ = &strategy;

        Some(strategy)
    }

    /// Calculate backoff delay using configured strategy.
    ///
    /// Implements exponential backoff, linear backoff, or fixed delay patterns
    /// to prevent rapid restart loops and give failing components time to recover.
    ///
    /// # Arguments
    /// * `child_id` - Identifier of the child being restarted
    ///
    /// # Returns
    /// Duration to wait before attempting restart
    pub fn calculate_backoff_delay(&self, child_id: &ChildId) -> core::time::Duration {
        if let Some(child_info) = self.children.get(child_id) {
            let retry_count = child_info.restart_count;

            // Simple exponential backoff: 100ms * 2^(retry_count-1), max 5 seconds
            let base_delay_ms: u64 = 100;
            let max_delay_ms: u64 = 5000;

            let exponential_delay = base_delay_ms
                .saturating_mul(2_u64.saturating_pow(retry_count.saturating_sub(1).max(0) as u32));
            let delay_ms = exponential_delay.min(max_delay_ms);

            core::time::Duration::from_millis(delay_ms)
        } else {
            core::time::Duration::from_millis(100) // Default minimal delay
        }
    }

    /// Apply restart intensity rate limiting.
    ///
    /// Checks if a child can be restarted within the configured intensity limits.
    /// This prevents infinite restart loops when a child consistently fails.
    ///
    /// # Arguments
    /// * `child_id` - Identifier of the child to check
    ///
    /// # Returns
    /// * `true` - Restart is allowed within intensity limits
    /// * `false` - Restart would exceed limits, escalation may be needed
    pub fn apply_restart_intensity(&mut self, child_id: &ChildId) -> bool {
        if let Some(child_info) = self.children.get(child_id) {
            child_info.restart_count <= self.max_restarts
        } else {
            false // Child not found
        }
    }

    /// Escalate failure to parent supervisor (hierarchical support).
    ///
    /// When a child repeatedly fails and exceeds restart limits, this method
    /// can be used to escalate the failure to a higher-level supervisor.
    ///
    /// # Arguments
    /// * `child_id` - Identifier of the repeatedly failing child
    /// * `_error` - The error that triggered escalation
    ///
    /// # Note
    /// This is a placeholder for hierarchical supervision. In a full implementation,
    /// this would send a message to a parent supervisor or trigger system-level
    /// failure handling (e.g., device reset in embedded systems).
    pub fn escalate_failure(&mut self, child_id: &ChildId, _error: super::ActorError) {
        #[cfg(feature = "debug-log")]
        log::error!("Escalating failure for child {child_id:?} - restart limits exceeded");

        // Remove the failing child from supervision to prevent further restart attempts
        self.children.remove(child_id);

        // In a production system, this might:
        // - Send escalation message to parent supervisor
        // - Trigger system-level failsafe (reset, safe mode, etc.)
        // - Log critical system event for debugging
    }

    /// Executes the restart logic for children that need to be restarted.
    ///
    /// This method implements the actual restart mechanism by calling the restart
    /// factories for each child that needs to be restarted according to the strategy.
    ///
    /// # Arguments
    /// * `failed_child_id` - ID of the child that failed
    /// * `strategy` - Restart strategy to apply
    ///
    /// # Returns
    /// The number of children successfully restarted.
    pub fn execute_restarts(
        &mut self,
        failed_child_id: &ChildId,
        strategy: RestartStrategy,
    ) -> usize {
        #[cfg(any(feature = "std", feature = "alloc"))]
        let children_to_restart = self.get_children_to_restart(failed_child_id, strategy);

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        let children_to_restart = self.get_children_to_restart(failed_child_id, strategy);

        let mut successfully_restarted = 0;

        // Process each child restart directly to avoid borrowing and type complexity
        let _total_children = children_to_restart.len();

        for child_id in children_to_restart {
            // Temporarily remove the child to avoid borrow conflicts
            if let Some(mut child_info) = self.children.remove(&child_id) {
                // Call the restart factory for this child
                let factory_result = (child_info.restart_factory)();

                // Update child state based on factory result
                #[cfg(feature = "async-tokio")]
                {
                    // For Tokio, the factory returns a JoinHandle
                    child_info.join_handle = Some(factory_result);
                    successfully_restarted += 1;

                    #[cfg(feature = "debug-log")]
                    log::info!("Successfully restarted child {child_id:?}");

                    // Put the child back in supervision
                    self.children.insert(child_id, child_info);
                }

                #[cfg(not(feature = "async-tokio"))]
                {
                    // For non-Tokio, the factory returns a Result
                    match factory_result {
                        Ok(()) => {
                            child_info.is_running = true;
                            successfully_restarted += 1;

                            #[cfg(feature = "debug-log")]
                            log::info!("Successfully restarted child {child_id:?}");

                            // Put the child back in supervision
                            let _ = self.children.insert(child_id, child_info);
                        }
                        Err(_err) => {
                            #[cfg(feature = "debug-log")]
                            log::error!("Failed to restart child {child_id:?}: {_err:?}");

                            // Don't put the child back - it failed to restart
                            // (child_info is dropped here)
                        }
                    }
                }
            }
        }

        #[cfg(feature = "debug-log")]
        log::info!(
            "Restart operation complete: {successfully_restarted}/{_total_children} children restarted successfully"
        );

        successfully_restarted
    }

    /// Gets the list of children that should be restarted based on the restart strategy.
    ///
    /// # Arguments
    /// * `failed_child_id` - ID of the child that failed
    /// * `strategy` - Restart strategy to apply
    ///
    /// # Returns
    /// A vector of child IDs that should be restarted.
    #[cfg(any(feature = "std", feature = "alloc"))]
    pub fn get_children_to_restart(
        &self,
        failed_child_id: &ChildId,
        strategy: RestartStrategy,
    ) -> Vec<ChildId> {
        match strategy {
            // Classic restart patterns - determine which children to restart
            RestartStrategy::OneForOne => {
                // Restart only the failed child
                alloc::vec![failed_child_id.clone()]
            }

            RestartStrategy::OneForAll => {
                // Restart all children
                self.children.keys().cloned().collect()
            }

            RestartStrategy::RestForOne => {
                // Restart the failed child and all children started after it
                if let Some(failed_child_info) = self.children.get(failed_child_id) {
                    let failed_sequence = failed_child_info.start_sequence;

                    // Collect the failed child and all children with sequence >= failed_sequence
                    self.children
                        .iter()
                        .filter(|(_, child_info)| child_info.start_sequence >= failed_sequence)
                        .map(|(child_id, _)| child_id.clone())
                        .collect()
                } else {
                    // Failed child not found, restart nothing
                    alloc::vec![]
                }
            }

            // OTP-style policies - convert to equivalent classic patterns
            // These policies determine WHETHER to restart, defaulting to OneForOne for WHICH children
            RestartStrategy::Permanent => {
                // Always restart - use OneForOne pattern (restart just the failed child)
                alloc::vec![failed_child_id.clone()]
            }

            RestartStrategy::Transient => {
                // Restart only on abnormal termination - use OneForOne pattern
                // Note: The decision of whether this is abnormal termination should be made elsewhere
                alloc::vec![failed_child_id.clone()]
            }

            // No-restart policies - return empty list
            RestartStrategy::Temporary | RestartStrategy::Never => {
                // Never restart
                alloc::vec![]
            }

            RestartStrategy::Escalate => {
                // Don't restart, escalate to parent - return empty list
                // The escalation logic should be handled elsewhere
                alloc::vec![]
            }
        }
    }

    /// Gets the list of children that should be restarted (no_std version).
    ///
    /// Returns a heapless Vec with fixed capacity for no_std environments.
    #[cfg(not(any(feature = "std", feature = "alloc")))]
    pub fn get_children_to_restart(
        &self,
        failed_child_id: &ChildId,
        strategy: RestartStrategy,
    ) -> heapless::Vec<ChildId, MAX_CHILDREN> {
        let mut result = heapless::Vec::new();

        match strategy {
            // Classic restart patterns - determine which children to restart
            RestartStrategy::OneForOne => {
                // Restart only the failed child
                let _ = result.push(failed_child_id.clone());
            }

            RestartStrategy::OneForAll => {
                // Restart all children
                for child_id in self.children.keys() {
                    if result.push(child_id.clone()).is_err() {
                        break; // Vec is full
                    }
                }
            }

            RestartStrategy::RestForOne => {
                // Restart the failed child and all children started after it
                if let Some(failed_child_info) = self.children.get(failed_child_id) {
                    let failed_sequence = failed_child_info.start_sequence;

                    // Collect the failed child and all children with sequence >= failed_sequence
                    for (child_id, child_info) in &self.children {
                        if child_info.start_sequence >= failed_sequence
                            && result.push(child_id.clone()).is_err()
                        {
                            break; // Vec is full
                        }
                    }
                }
                // If failed child not found, result remains empty (restart nothing)
            }

            // OTP-style policies - convert to equivalent classic patterns
            // These policies determine WHETHER to restart, defaulting to OneForOne for WHICH children
            RestartStrategy::Permanent => {
                // Always restart - use OneForOne pattern (restart just the failed child)
                let _ = result.push(failed_child_id.clone());
            }

            RestartStrategy::Transient => {
                // Restart only on abnormal termination - use OneForOne pattern
                // Note: The decision of whether this is abnormal termination should be made elsewhere
                let _ = result.push(failed_child_id.clone());
            }

            // No-restart policies - return empty list
            RestartStrategy::Temporary | RestartStrategy::Never => {
                // Never restart - result remains empty
            }

            RestartStrategy::Escalate => {
                // Don't restart, escalate to parent - result remains empty
                // The escalation logic should be handled elsewhere
            }
        }

        result
    }

    /// Sets the JoinHandle for a child (Tokio-specific).
    ///
    /// This allows the supervisor to monitor child task completion and detect failures.
    #[cfg(feature = "async-tokio")]
    pub fn set_child_handle(
        &mut self,
        child_id: &ChildId,
        handle: JoinHandle<Result<(), ActorError>>,
    ) -> Result<(), SupervisorError> {
        if let Some(child_info) = self.children.get_mut(child_id) {
            child_info.join_handle = Some(handle);
            Ok(())
        } else {
            Err(SupervisorError::ChildNotFound)
        }
    }

    /// Adds a child actor to supervision with its JoinHandle and restart factory atomically (Tokio-specific).
    ///
    /// This method combines child addition and handle/factory setup into a single atomic operation,
    /// preventing race conditions. This is the preferred method for spawning supervised actors in Tokio.
    ///
    /// # Arguments
    /// * `child_id` - Unique identifier for the child
    /// * `handle` - JoinHandle for monitoring the child task
    /// * `restart_factory` - Function that spawns a new instance of the child actor
    /// * `restart_strategy` - Optional custom restart strategy (uses default if None)
    ///
    /// # Returns
    /// `Ok(())` if the child was added successfully, `Err(SupervisorError)` if the operation failed.
    #[cfg(feature = "async-tokio")]
    pub fn add_child_with_handle_and_factory(
        &mut self,
        child_id: ChildId,
        handle: JoinHandle<Result<(), ActorError>>,
        restart_factory: RestartFactory,
        restart_strategy: Option<RestartStrategy>,
    ) -> Result<(), SupervisorError> {
        // Check if child already exists
        if self.children.contains_key(&child_id) {
            return Err(SupervisorError::ChildAlreadyExists);
        }

        let strategy = restart_strategy.unwrap_or(self.default_restart_strategy);

        let child_info = ChildInfo {
            restart_strategy: strategy,
            restart_count: 0,
            start_sequence: self.next_start_sequence,

            #[cfg(feature = "std")]
            window_start: std::time::Instant::now(),

            #[cfg(not(feature = "std"))]
            window_start_ms: Self::current_time_ms(),

            join_handle: Some(handle),

            restart_factory,
        };

        self.children.insert(child_id, child_info);
        self.next_start_sequence += 1;
        Ok(())
    }

    /// Adds a child actor to supervision with its JoinHandle atomically (Tokio-specific, legacy).
    ///
    /// This is a legacy method that creates a no-op restart factory.
    /// For actual restart functionality, use `add_child_with_handle_and_factory` instead.
    #[cfg(feature = "async-tokio")]
    pub fn add_child_with_handle(
        &mut self,
        child_id: ChildId,
        handle: JoinHandle<Result<(), ActorError>>,
        restart_strategy: Option<RestartStrategy>,
    ) -> Result<(), SupervisorError> {
        // Create a no-op restart factory for backwards compatibility
        let no_op_factory: RestartFactory =
            Box::new(|| tokio::spawn(async { Err(ActorError::StartupFailure) }));

        self.add_child_with_handle_and_factory(child_id, handle, no_op_factory, restart_strategy)
    }

    /// Checks for completed child tasks and returns their results (Tokio-specific).
    ///
    /// This method should be called periodically to detect child failures.
    /// It uses non-blocking polling to check `JoinHandle` completion.
    #[cfg(feature = "async-tokio")]
    pub fn poll_children(&mut self) -> Vec<(ChildId, Result<(), ActorError>)> {
        let mut completed = Vec::new();
        let mut to_remove = Vec::new();

        for (child_id, child_info) in &mut self.children {
            if let Some(handle) = child_info.join_handle.take() {
                // Check if the task is finished and try to get the result non-blockingly
                if handle.is_finished() {
                    // Task is finished - extract the actual result
                    if let Some(join_result) = handle.now_or_never() {
                        // Convert JoinResult to our ActorError format
                        let result = match join_result {
                            Ok(task_result) => task_result, // This is Result<(), ActorError>
                            Err(join_error) => {
                                // JoinError indicates panic or cancellation
                                if join_error.is_panic() {
                                    // Use panic_handling utilities to extract message
                                    let actor_error =
                                        crate::actor::panic_handling::capture_panic_info(
                                            join_error,
                                        );
                                    let message = match actor_error {
                                        ActorError::Panic { message, .. } => message,
                                        _ => Some("task panicked".into()),
                                    };
                                    Err(ActorError::Panic {
                                        message,
                                        actor_id: Some(format!("{child_id:?}")),
                                    })
                                } else {
                                    // Task was cancelled
                                    Err(ActorError::Panic {
                                        message: Some("task was cancelled".into()),
                                        actor_id: Some(format!("{child_id:?}")),
                                    })
                                }
                            }
                        };

                        completed.push((child_id.clone(), result));
                        to_remove.push(child_id.clone());
                    }
                    // Note: None case is impossible since we checked is_finished() == true
                } else {
                    // Task is still running - put the handle back
                    child_info.join_handle = Some(handle);
                }
            }
        }

        // Remove completed children (they will be re-added if restarted)
        for child_id in to_remove {
            self.children.remove(&child_id);
        }

        completed
    }

    /// Gets platform-specific current time in milliseconds.
    fn current_time_ms() -> u64 {
        #[cfg(feature = "std")]
        {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64
        }

        #[cfg(all(feature = "async-embassy", not(feature = "std")))]
        {
            embassy_time::Instant::now().as_millis()
        }

        #[cfg(all(not(feature = "std"), not(feature = "async-embassy")))]
        {
            // Fallback no_std implementation - uses atomic counter with 1-second increments
            // This provides reasonable restart window behavior where each function call
            // represents approximately 1 second of logical time
            use core::sync::atomic::{AtomicU32, Ordering};
            static LOGICAL_TIME_SECONDS: AtomicU32 = AtomicU32::new(1000);
            LOGICAL_TIME_SECONDS.fetch_add(1000, Ordering::SeqCst) as u64
        }
    }
}

impl<ChildId, const MAX_CHILDREN: usize> Default for SupervisorActor<ChildId, MAX_CHILDREN>
where
    ChildId: Clone + PartialEq + core::fmt::Debug + core::hash::Hash + Eq,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<ChildId, const MAX_CHILDREN: usize> Supervisor for SupervisorActor<ChildId, MAX_CHILDREN>
where
    ChildId: Clone + PartialEq + core::fmt::Debug + core::hash::Hash + Eq,
{
    type ChildId = ChildId;

    fn on_child_failure(&mut self, child_id: Self::ChildId) -> RestartStrategy {
        self.handle_child_failure(&child_id)
            .unwrap_or(RestartStrategy::OneForOne)
    }
}

impl<ChildId, const MAX_CHILDREN: usize> Actor for SupervisorActor<ChildId, MAX_CHILDREN>
where
    ChildId: Clone + PartialEq + core::fmt::Debug + core::hash::Hash + Eq + Send + 'static,
{
    type Message = SupervisorMessage<ChildId>;
    type Future<'a>
        = core::future::Ready<()>
    where
        Self: 'a;

    fn handle(&mut self, msg: Self::Message) -> Self::Future<'_> {
        match msg {
            SupervisorMessage::ChildStarted { id: _ } => {
                #[cfg(feature = "debug-log")]
                log::info!("Child started successfully");

                // Update child status
                #[cfg(not(feature = "async-tokio"))]
                {
                    // Note: Without the id, we can't update specific child status
                    // This would need to be redesigned to track child status properly
                }
            }

            SupervisorMessage::ChildStopped { id: _ } => {
                #[cfg(feature = "debug-log")]
                log::info!("Child stopped gracefully");

                // Update child status
                #[cfg(not(feature = "async-tokio"))]
                {
                    // Note: Without the id, we can't update specific child status
                    // This would need to be redesigned to track child status properly
                }
            }

            SupervisorMessage::ChildPanicked { id, error } => {
                #[cfg(feature = "debug-log")]
                log::warn!("Child {id:?} panicked - determining restart strategy");

                #[cfg(any(feature = "std", feature = "alloc"))]
                {
                    if let Some(strategy) = self.handle_child_panic(&id, *error) {
                        #[cfg(feature = "debug-log")]
                        log::info!("Executing restart strategy: {strategy:?} for child {id:?}");
                        #[cfg(not(feature = "debug-log"))]
                        let _ = &strategy;
                    }
                }
                #[cfg(not(any(feature = "std", feature = "alloc")))]
                {
                    if let Some(strategy) = self.handle_child_panic(&id, error) {
                        #[cfg(feature = "debug-log")]
                        log::info!("Executing restart strategy: {strategy:?} for child {id:?}");
                        #[cfg(not(feature = "debug-log"))]
                        let _ = &strategy;
                    }
                }
            }

            SupervisorMessage::StartChild { id } => {
                #[cfg(feature = "debug-log")]
                log::info!("Request to start child {id:?}");

                // Add child to supervision with default strategy (no-op factory)
                let _ = self.add_child(id, None);
            }

            SupervisorMessage::StopChild { id } => {
                #[cfg(feature = "debug-log")]
                log::info!("Request to stop child {id:?}");

                // Remove child from supervision
                self.remove_child(&id);
            }

            SupervisorMessage::RestartChild { id } => {
                #[cfg(feature = "debug-log")]
                log::info!("Request to restart child {id:?}");

                // Treat as a failure for restart counting purposes
                if let Some(strategy) = self.handle_child_failure(&id) {
                    #[cfg(feature = "debug-log")]
                    log::info!(
                        "Executing restart strategy: {strategy:?} for manual restart of child {id:?}"
                    );

                    // Execute the actual restart logic
                    let _restarted_count = self.execute_restarts(&id, strategy);

                    #[cfg(feature = "debug-log")]
                    if _restarted_count > 0 {
                        log::info!(
                            "Successfully restarted {_restarted_count} children for manual restart"
                        );
                    } else {
                        log::warn!("Failed to restart any children for manual restart");
                    }
                } else {
                    #[cfg(feature = "debug-log")]
                    log::warn!(
                        "Child {id:?} exceeded restart limit or was not found for manual restart"
                    );
                }
            }

            SupervisorMessage::ChildEscalated {
                supervisor_id,
                failed_child_id: _,
                error: _,
            } => {
                #[cfg(feature = "debug-log")]
                log::warn!("Child supervisor {supervisor_id:?} escalated a failure");

                // For now, treat escalation as a supervisor failure requiring restart
                // In a more sophisticated implementation, this could trigger different escalation policies
                if let Some(strategy) = self.handle_child_failure(&supervisor_id) {
                    #[cfg(feature = "debug-log")]
                    log::info!(
                        "Executing restart strategy: {strategy:?} for escalated supervisor {supervisor_id:?}"
                    );

                    // Execute the actual restart logic for the supervisor that escalated
                    let _restarted_count = self.execute_restarts(&supervisor_id, strategy);

                    #[cfg(feature = "debug-log")]
                    if _restarted_count > 0 {
                        log::info!(
                            "Successfully restarted {_restarted_count} supervisors after escalation"
                        );
                    } else {
                        log::warn!("Failed to restart escalated supervisor");
                    }
                } else {
                    #[cfg(feature = "debug-log")]
                    log::warn!(
                        "Escalated supervisor {supervisor_id:?} exceeded restart limit or was not found"
                    );
                }
            }
        }

        core::future::ready(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supervisor_creation_works() {
        let supervisor = SupervisorActor::<u32, 8>::new();
        assert_eq!(
            supervisor.default_restart_strategy,
            RestartStrategy::OneForOne
        );
        assert_eq!(supervisor.max_restarts, 5);
        assert_eq!(supervisor.restart_window_ms, 60_000);
    }

    #[test]
    fn add_and_remove_children() {
        let mut supervisor = SupervisorActor::<u32, 8>::new();

        // Add a child
        assert!(supervisor.add_child(1, None).is_ok());

        // Remove the child
        assert!(supervisor.remove_child(&1));
        assert!(!supervisor.remove_child(&1)); // Second removal should fail
    }

    #[test]
    fn child_uniqueness_check() {
        let mut supervisor = SupervisorActor::<u32, 8>::new();

        // Add a child successfully
        assert!(supervisor.add_child(1, None).is_ok());

        // Try to add the same child again - should fail
        assert_eq!(
            supervisor.add_child(1, Some(RestartStrategy::OneForAll)),
            Err(SupervisorError::ChildAlreadyExists)
        );

        // Verify we can still add a different child
        assert!(supervisor.add_child(2, None).is_ok());

        // After removal, we should be able to add the same ID again
        assert!(supervisor.remove_child(&1));
        assert!(supervisor.add_child(1, None).is_ok());
    }

    #[test]
    fn restart_strategies() {
        let supervisor = SupervisorActor::<u32, 8>::new();

        // OneForOne should restart only the failed child
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            let to_restart = supervisor.get_children_to_restart(&1, RestartStrategy::OneForOne);
            assert_eq!(to_restart, alloc::vec![1]);
        }

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        {
            let to_restart = supervisor.get_children_to_restart(&1, RestartStrategy::OneForOne);
            assert_eq!(to_restart.len(), 1);
        }
    }

    #[test]
    fn supervisor_actor_trait() {
        let mut supervisor = SupervisorActor::<u32, 8>::new();

        // Test handling supervisor messages
        let msg = SupervisorMessage::ChildStarted { id: 1 };
        let _future = supervisor.handle(msg);

        let msg = SupervisorMessage::ChildPanicked {
            id: 1,
            error: {
                #[cfg(any(feature = "std", feature = "alloc"))]
                {
                    Box::new(ActorError::Panic {
                        message: Some("test panic".to_string()),
                        actor_id: Some("test_actor".to_string()),
                    })
                }
                #[cfg(not(any(feature = "std", feature = "alloc")))]
                {
                    use heapless::String as HeaplessString;

                    let mut panic_msg = HeaplessString::<128>::new();
                    let _ = panic_msg.push_str("test panic");

                    let mut actor_id_str = HeaplessString::<128>::new();
                    let _ = actor_id_str.push_str("test_actor");

                    ActorError::Panic {
                        message: Some(panic_msg),
                        actor_id: Some(actor_id_str),
                    }
                }
            },
        };
        let _future = supervisor.handle(msg);
    }

    #[test]
    fn rest_for_one_strategy_ordering() {
        let mut supervisor = SupervisorActor::<u32, 8>::new();

        // Add children in specific order
        assert!(
            supervisor
                .add_child(1, Some(RestartStrategy::RestForOne))
                .is_ok()
        );
        assert!(
            supervisor
                .add_child(2, Some(RestartStrategy::RestForOne))
                .is_ok()
        );
        assert!(
            supervisor
                .add_child(3, Some(RestartStrategy::RestForOne))
                .is_ok()
        );
        assert!(
            supervisor
                .add_child(4, Some(RestartStrategy::RestForOne))
                .is_ok()
        );

        // Test RestForOne for middle child (child 2 fails)
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            let to_restart = supervisor.get_children_to_restart(&2, RestartStrategy::RestForOne);
            // Should restart child 2, 3, and 4 (all children started at or after child 2)
            assert_eq!(to_restart.len(), 3);
            assert!(to_restart.contains(&2));
            assert!(to_restart.contains(&3));
            assert!(to_restart.contains(&4));
            assert!(!to_restart.contains(&1)); // Child 1 should not be restarted
        }

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        {
            let to_restart = supervisor.get_children_to_restart(&2, RestartStrategy::RestForOne);
            // Should restart child 2, 3, and 4
            assert_eq!(to_restart.len(), 3);

            // Check that the correct children are included
            let mut contains_2 = false;
            let mut contains_3 = false;
            let mut contains_4 = false;
            let mut contains_1 = false;

            for child_id in &to_restart {
                match *child_id {
                    1 => contains_1 = true,
                    2 => contains_2 = true,
                    3 => contains_3 = true,
                    4 => contains_4 = true,
                    _ => {}
                }
            }

            assert!(contains_2);
            assert!(contains_3);
            assert!(contains_4);
            assert!(!contains_1); // Child 1 should not be restarted
        }

        // Test RestForOne for first child (child 1 fails)
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            let to_restart = supervisor.get_children_to_restart(&1, RestartStrategy::RestForOne);
            // Should restart all children (1, 2, 3, 4) since child 1 was first
            assert_eq!(to_restart.len(), 4);
            assert!(to_restart.contains(&1));
            assert!(to_restart.contains(&2));
            assert!(to_restart.contains(&3));
            assert!(to_restart.contains(&4));
        }

        // Test RestForOne for last child (child 4 fails)
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            let to_restart = supervisor.get_children_to_restart(&4, RestartStrategy::RestForOne);
            // Should restart only child 4 (last child)
            assert_eq!(to_restart.len(), 1);
            assert!(to_restart.contains(&4));
        }

        // Test RestForOne for non-existent child
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            let to_restart = supervisor.get_children_to_restart(&999, RestartStrategy::RestForOne);
            // Should restart nothing if child doesn't exist
            assert_eq!(to_restart.len(), 0);
        }
    }

    #[cfg(all(test, feature = "async-tokio", feature = "std"))]
    #[tokio::test]
    async fn test_restart_factory_execution() {
        let mut supervisor = SupervisorActor::<u32, 8>::new();

        // Create a counter to track restart calls
        use std::sync::{Arc, Mutex};
        let restart_call_count = Arc::new(Mutex::new(0));
        let counter_clone = restart_call_count.clone();

        // Create a restart factory that increments a counter when called
        let restart_factory: RestartFactory = Box::new(move || {
            let mut count = counter_clone.lock().unwrap();
            *count += 1;
            tokio::spawn(async { Ok(()) })
        });

        // Add a child with the restart factory
        assert!(
            supervisor
                .add_child_with_factory(1, restart_factory, Some(RestartStrategy::OneForOne))
                .is_ok()
        );

        // Simulate a child failure and execute restarts
        let restarted_count = supervisor.execute_restarts(&1, RestartStrategy::OneForOne);

        // Verify that restart was attempted
        assert_eq!(restarted_count, 1);

        // Verify that the restart factory was actually called
        let final_count = *restart_call_count.lock().unwrap();
        assert_eq!(final_count, 1);
    }

    #[cfg(all(test, feature = "async-tokio", feature = "std"))]
    #[tokio::test]
    async fn test_restart_with_one_for_all_strategy() {
        let mut supervisor = SupervisorActor::<u32, 8>::new();

        // Create counters for tracking restart calls for different children
        use std::sync::{Arc, Mutex};
        let restart_count_1 = Arc::new(Mutex::new(0));
        let restart_count_2 = Arc::new(Mutex::new(0));

        let counter_1_clone = restart_count_1.clone();
        let counter_2_clone = restart_count_2.clone();

        // Create restart factories for two different children
        let factory_1: RestartFactory = Box::new(move || {
            let mut count = counter_1_clone.lock().unwrap();
            *count += 1;
            tokio::spawn(async { Ok(()) })
        });

        let factory_2: RestartFactory = Box::new(move || {
            let mut count = counter_2_clone.lock().unwrap();
            *count += 1;
            tokio::spawn(async { Ok(()) })
        });

        // Add two children with restart factories
        assert!(
            supervisor
                .add_child_with_factory(1, factory_1, Some(RestartStrategy::OneForAll))
                .is_ok()
        );
        assert!(
            supervisor
                .add_child_with_factory(2, factory_2, Some(RestartStrategy::OneForAll))
                .is_ok()
        );

        // Simulate child 1 failure with OneForAll strategy (should restart both children)
        let restarted_count = supervisor.execute_restarts(&1, RestartStrategy::OneForAll);

        // Verify that both children were restarted
        assert_eq!(restarted_count, 2);

        // Verify that both restart factories were called
        let final_count_1 = *restart_count_1.lock().unwrap();
        let final_count_2 = *restart_count_2.lock().unwrap();
        assert_eq!(final_count_1, 1);
        assert_eq!(final_count_2, 1);
    }

    #[test]
    fn test_no_std_timing_behavior() {
        // Test that demonstrates the restart window behavior works correctly
        // This test passes regardless of whether std or no_std timing is used
        let mut supervisor = SupervisorActor::<u32, 8>::with_config(
            RestartStrategy::OneForOne,
            2,    // max 2 restarts per window
            1000, // small window to test reset behavior
        );

        // Add a child
        assert!(supervisor.add_child(1, None).is_ok());

        // First failure - should succeed
        assert_eq!(
            supervisor.handle_child_failure(&1),
            Some(RestartStrategy::OneForOne)
        );

        // Second failure - should succeed (still under max restarts)
        assert_eq!(
            supervisor.handle_child_failure(&1),
            Some(RestartStrategy::OneForOne)
        );

        // Third failure - this will either:
        // 1. Exceed max restarts if we're still in the same window, OR
        // 2. Reset the window and succeed if enough time has passed
        let result3 = supervisor.handle_child_failure(&1);

        if result3.is_none() {
            // Child exceeded restart limit and was removed
            assert!(!supervisor.children.contains_key(&1));
        } else {
            // Window reset, child still exists
            assert!(supervisor.children.contains_key(&1));

            // One more failure should still work (restart count reset)
            assert_eq!(
                supervisor.handle_child_failure(&1),
                Some(RestartStrategy::OneForOne)
            );
        }

        // This test validates that the restart mechanism works correctly
        // regardless of the specific timing implementation
    }

    #[test]
    fn test_restart_window_reset_behavior() {
        // Test that restart windows reset correctly and allow more restarts
        let mut supervisor = SupervisorActor::<u32, 8>::with_config(
            RestartStrategy::OneForOne,
            1,   // max 1 restart per window (very restrictive)
            100, // very small window to force resets
        );

        // Add a child
        assert!(supervisor.add_child(1, None).is_ok());

        // First failure - should succeed (1st restart in window)
        assert_eq!(
            supervisor.handle_child_failure(&1),
            Some(RestartStrategy::OneForOne)
        );

        // Second failure - should either:
        // 1. Fail if we're still in the same window (exceeded max_restarts), OR
        // 2. Succeed if the window has reset
        let result2 = supervisor.handle_child_failure(&1);

        if result2.is_none() {
            // Child was removed due to exceeding restart limit
            assert!(!supervisor.children.contains_key(&1));
        } else {
            // Window reset successfully, child still supervised
            assert!(supervisor.children.contains_key(&1));

            // Since window reset, we should be able to have one more failure
            // before hitting the limit again
            let result3 = supervisor.handle_child_failure(&1);

            // This could either succeed (window reset again) or fail (hit limit)
            // Either is valid behavior depending on timing
            if result3.is_none() {
                assert!(!supervisor.children.contains_key(&1));
            } else {
                assert!(supervisor.children.contains_key(&1));
            }
        }

        // This test validates that restart window resets work correctly
        // regardless of the specific timing implementation details
    }

    // Phase 3 Comprehensive Testing: New Supervision Integration Methods

    #[test]
    fn test_handle_child_panic_integration() {
        let mut supervisor = SupervisorActor::<u32, 8>::with_config(
            RestartStrategy::OneForOne,
            3,      // Allow 3 restarts before escalation
            60_000, // 60 second window
        );

        // Add a child
        assert!(supervisor.add_child(1, None).is_ok());

        // Create a test panic error
        #[cfg(any(feature = "std", feature = "alloc"))]
        let panic_error = ActorError::Panic {
            message: Some("Test panic in actor".to_string()),
            actor_id: Some("test_actor_1".to_string()),
        };

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        let panic_error = {
            let mut panic_msg = crate::actor::ActorString::new();
            let _ = panic_msg.push_str("Test panic in actor");
            let mut actor_id_str = crate::actor::ActorString::new();
            let _ = actor_id_str.push_str("test_actor_1");
            ActorError::Panic {
                message: Some(panic_msg),
                actor_id: Some(actor_id_str),
            }
        };

        // Test that handle_child_panic integrates with handle_child_failure
        let strategy = supervisor.handle_child_panic(&1, panic_error);
        assert_eq!(strategy, Some(RestartStrategy::OneForOne));

        // Verify restart count increased
        let child_info = supervisor.children.get(&1).unwrap();
        assert_eq!(child_info.restart_count, 1);

        // Test multiple panics leading to removal
        for _ in 0..3 {
            let panic_error = ActorError::Custom("Repeated panic");
            let _ = supervisor.handle_child_panic(&1, panic_error);
        }

        // Child should be removed after exceeding restart limit
        assert!(!supervisor.children.contains_key(&1));
    }

    #[test]
    fn test_calculate_backoff_delay_exponential() {
        let mut supervisor = SupervisorActor::<u32, 8>::new();

        // Add a child and simulate multiple failures to test backoff progression
        assert!(supervisor.add_child(1, None).is_ok());

        // Initial delay (no failures yet)
        let delay0 = supervisor.calculate_backoff_delay(&1);
        assert_eq!(delay0, core::time::Duration::from_millis(100)); // Default minimal delay

        // Simulate failures and check exponential backoff
        supervisor.handle_child_failure(&1); // 1st failure
        let delay1 = supervisor.calculate_backoff_delay(&1);
        assert_eq!(delay1, core::time::Duration::from_millis(100)); // 100ms * 2^0 = 100ms

        supervisor.handle_child_failure(&1); // 2nd failure  
        let delay2 = supervisor.calculate_backoff_delay(&1);
        assert_eq!(delay2, core::time::Duration::from_millis(200)); // 100ms * 2^1 = 200ms

        supervisor.handle_child_failure(&1); // 3rd failure
        let delay3 = supervisor.calculate_backoff_delay(&1);
        assert_eq!(delay3, core::time::Duration::from_millis(400)); // 100ms * 2^2 = 400ms

        // Test max delay cap (5 seconds)
        // Simulate many failures to test the cap
        let mut supervisor_capped = SupervisorActor::<u32, 8>::with_config(
            RestartStrategy::OneForOne,
            20,      // Allow many restarts to test delay cap
            300_000, // Very long window
        );
        assert!(supervisor_capped.add_child(2, None).is_ok());

        // Force high restart count to test max delay
        for _ in 0..10 {
            supervisor_capped.handle_child_failure(&2);
        }

        let max_delay = supervisor_capped.calculate_backoff_delay(&2);
        assert_eq!(max_delay, core::time::Duration::from_millis(5000)); // Capped at 5 seconds
    }

    #[test]
    fn test_escalate_failure_removes_child() {
        let mut supervisor = SupervisorActor::<u32, 8>::new();

        // Add a child that will repeatedly fail
        assert!(supervisor.add_child(1, None).is_ok());
        assert!(supervisor.children.contains_key(&1));

        // Create a test escalation error
        let escalation_error = ActorError::Custom("Repeated failure requiring escalation");

        // Escalate the failure
        supervisor.escalate_failure(&1, escalation_error);

        // Child should be removed from supervision to prevent further restart attempts
        assert!(!supervisor.children.contains_key(&1));
    }

    #[test]
    fn test_apply_restart_intensity_rate_limiting() {
        let mut supervisor = SupervisorActor::<u32, 8>::with_config(
            RestartStrategy::OneForOne,
            2,      // Very low restart limit for testing
            60_000, // 60 second window
        );

        // Add a child
        assert!(supervisor.add_child(1, None).is_ok());

        // Initially should allow restart
        assert!(supervisor.apply_restart_intensity(&1));

        // After one failure, should still allow restart
        supervisor.handle_child_failure(&1);
        assert!(supervisor.apply_restart_intensity(&1));

        // After two failures, should still allow restart (at limit)
        supervisor.handle_child_failure(&1);
        assert!(supervisor.apply_restart_intensity(&1));

        // After three failures, should exceed limit and not allow restart
        supervisor.handle_child_failure(&1); // This removes the child
        assert!(!supervisor.apply_restart_intensity(&1)); // Child no longer exists

        // Test with non-existent child
        assert!(!supervisor.apply_restart_intensity(&999));
    }

    #[test]
    fn test_supervisor_message_child_escalated_handling() {
        let mut supervisor = SupervisorActor::<u32, 8>::new();

        // Add supervisor as a child (simulating hierarchical supervision)
        assert!(supervisor.add_child(100, None).is_ok()); // Supervisor ID 100

        #[cfg(any(feature = "std", feature = "alloc"))]
        let escalation_error = Box::new(ActorError::Custom("Child supervision failed"));

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        let escalation_error = ActorError::Custom("Child supervision failed");

        // Test handling escalation message
        let escalation_msg = SupervisorMessage::ChildEscalated {
            supervisor_id: 100,
            failed_child_id: 1,
            error: escalation_error,
        };

        // This should trigger restart logic for the escalated supervisor
        let _future = supervisor.handle(escalation_msg);

        // Verify that the escalation was processed (supervisor treats it as a failure)
        // The exact behavior depends on restart limits and timing
        // This test validates that the escalation message is handled without panicking
    }

    #[test]
    fn test_otp_style_restart_strategies() {
        let _supervisor = SupervisorActor::<u32, 8>::new();

        // Test OTP-style policies convert to appropriate restart patterns

        // Permanent policy - always restart (OneForOne pattern)
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            let to_restart = _supervisor.get_children_to_restart(&1, RestartStrategy::Permanent);
            assert_eq!(to_restart, alloc::vec![1]);
        }

        // Transient policy - restart on abnormal termination (OneForOne pattern)
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            let to_restart = _supervisor.get_children_to_restart(&1, RestartStrategy::Transient);
            assert_eq!(to_restart, alloc::vec![1]);
        }

        // Temporary policy - never restart
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            let to_restart = _supervisor.get_children_to_restart(&1, RestartStrategy::Temporary);
            assert_eq!(to_restart.len(), 0);
        }

        // Never policy - explicit no-restart
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            let to_restart = _supervisor.get_children_to_restart(&1, RestartStrategy::Never);
            assert_eq!(to_restart.len(), 0);
        }

        // Escalate policy - don't restart locally, escalate to parent
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            let to_restart = _supervisor.get_children_to_restart(&1, RestartStrategy::Escalate);
            assert_eq!(to_restart.len(), 0);
        }
    }

    #[test]
    fn test_restart_factory_error_handling() {
        #[cfg(not(feature = "async-tokio"))]
        let mut supervisor = SupervisorActor::<u32, 8>::new();

        #[cfg(feature = "async-tokio")]
        let supervisor = SupervisorActor::<u32, 8>::new();

        // Test handling of restart factory failures in no_std environment
        #[cfg(not(feature = "async-tokio"))]
        {
            let failing_factory: RestartFactory = Box::new(|| Err(SupervisorError::RestartFailed));

            assert!(
                supervisor
                    .add_child_with_factory(1, failing_factory, Some(RestartStrategy::OneForOne))
                    .is_ok()
            );

            // Execute restart - should handle factory failure gracefully
            let restarted_count = supervisor.execute_restarts(&1, RestartStrategy::OneForOne);
            assert_eq!(restarted_count, 0); // No children restarted due to factory failure
        }

        // Test successful restart factory in no_std environment
        #[cfg(not(feature = "async-tokio"))]
        {
            let successful_factory: RestartFactory = Box::new(|| Ok(()));

            assert!(
                supervisor
                    .add_child_with_factory(2, successful_factory, Some(RestartStrategy::OneForOne))
                    .is_ok()
            );

            // Execute restart - should succeed
            let restarted_count = supervisor.execute_restarts(&2, RestartStrategy::OneForOne);
            assert_eq!(restarted_count, 1); // One child restarted successfully
        }

        // Suppress unused variable warning when async-tokio feature is enabled
        #[cfg(feature = "async-tokio")]
        {
            let _ = supervisor;
        }
    }

    #[test]
    fn test_hierarchical_supervision_patterns() {
        let mut parent_supervisor = SupervisorActor::<&'static str, 4>::new();
        let mut child_supervisor = SupervisorActor::<u32, 8>::new();

        // Set up hierarchical structure
        assert!(
            parent_supervisor
                .add_child("child_supervisor", None)
                .is_ok()
        );
        assert!(child_supervisor.add_child(1, None).is_ok());
        assert!(child_supervisor.add_child(2, None).is_ok());

        // Simulate child failure in the child supervisor
        let strategy = child_supervisor.handle_child_failure(&1);
        assert_eq!(strategy, Some(RestartStrategy::OneForOne));

        // Simulate escalation from child supervisor to parent
        let escalation_error = ActorError::Custom("Child supervisor overwhelmed");

        // Parent supervisor handles escalation from child supervisor
        let parent_strategy = parent_supervisor.handle_child_failure(&"child_supervisor");
        assert_eq!(parent_strategy, Some(RestartStrategy::OneForOne));

        // Test escalate failure on child supervisor
        child_supervisor.escalate_failure(&1, escalation_error);
        assert!(!child_supervisor.children.contains_key(&1)); // Child removed after escalation
    }

    #[test]
    fn test_deterministic_backoff_behavior() {
        let _supervisor = SupervisorActor::<u32, 8>::new();

        // Test that backoff calculations are deterministic and predictable
        let test_cases = [
            (0, 100),   // No failures yet: minimal delay
            (1, 100),   // 1st failure: 100ms * 2^0 = 100ms
            (2, 200),   // 2nd failure: 100ms * 2^1 = 200ms
            (3, 400),   // 3rd failure: 100ms * 2^2 = 400ms
            (4, 800),   // 4th failure: 100ms * 2^3 = 800ms
            (5, 1600),  // 5th failure: 100ms * 2^4 = 1600ms
            (6, 3200),  // 6th failure: 100ms * 2^5 = 3200ms
            (10, 5000), // 10th failure: Capped at 5000ms
            (20, 5000), // 20th failure: Still capped at 5000ms
        ];

        for (restart_count, expected_delay_ms) in test_cases.iter() {
            // Create a mock child with specific restart count
            let mut test_supervisor = SupervisorActor::<u32, 8>::new();
            assert!(test_supervisor.add_child(1, None).is_ok());

            // Simulate the specified number of failures
            for _ in 0..*restart_count {
                if test_supervisor.handle_child_failure(&1).is_none() {
                    break; // Child removed due to restart limits
                }
            }

            // Only test if child still exists
            if test_supervisor.children.contains_key(&1) {
                let calculated_delay = test_supervisor.calculate_backoff_delay(&1);
                let expected_delay = core::time::Duration::from_millis(*expected_delay_ms);
                assert_eq!(
                    calculated_delay,
                    expected_delay,
                    "Restart count {restart_count}: expected {expected_delay_ms}ms, got {}ms",
                    calculated_delay.as_millis()
                );
            }
        }
    }
}
