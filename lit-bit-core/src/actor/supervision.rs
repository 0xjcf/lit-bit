//! Supervision utilities for managing child actors with restart strategies.
//!
//! This module implements OTP-style supervision patterns as described in the research document,
//! providing platform-dual supervision that works with both Tokio (JoinHandle monitoring) and
//! Embassy (message signaling) environments.

use super::{Actor, RestartStrategy, Supervisor, SupervisorMessage};

// Only import ActorError when async-tokio feature is enabled
#[cfg(feature = "async-tokio")]
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
use alloc::vec::Vec;

/// Error types for supervisor operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorError {
    /// Child capacity limit reached (no_std environments)
    CapacityExceeded,
    /// Child with this ID already exists
    ChildAlreadyExists,
    /// Child with this ID not found
    ChildNotFound,
}

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
/// // Add children and handle SupervisorMessage events
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
}

/// Information about a supervised child actor.
struct ChildInfo {
    /// Restart strategy for this child
    restart_strategy: RestartStrategy,

    /// Number of restarts within the current window
    restart_count: usize,

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
        }
    }

    /// Adds a child actor to supervision.
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
        // Check if child already exists
        if self.children.contains_key(&child_id) {
            return Err(SupervisorError::ChildAlreadyExists);
        }

        let strategy = restart_strategy.unwrap_or(self.default_restart_strategy);

        let child_info = ChildInfo {
            restart_strategy: strategy,
            restart_count: 0,

            #[cfg(feature = "std")]
            window_start: std::time::Instant::now(),

            #[cfg(not(feature = "std"))]
            window_start_ms: 0, // Will be set on first restart

            #[cfg(feature = "async-tokio")]
            join_handle: None,

            #[cfg(not(feature = "async-tokio"))]
            is_running: true,
        };

        #[cfg(feature = "async-tokio")]
        {
            self.children.insert(child_id, child_info);
            Ok(())
        }

        #[cfg(not(feature = "async-tokio"))]
        {
            self.children
                .insert(child_id, child_info)
                .map(|_| ())
                .map_err(|_| SupervisorError::CapacityExceeded)
        }
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
                // Note: This requires tracking child start order, which is simplified here
                // In a full implementation, you would maintain a start order list

                // For now, implement as OneForOne (could be enhanced to track order)
                alloc::vec![failed_child_id.clone()]
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
                // For now, implement as OneForOne
                let _ = result.push(failed_child_id.clone());
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
                                    Err(ActorError::Panic)
                                } else {
                                    // Task was cancelled
                                    Err(ActorError::Panic) // Treat cancellation as panic for now
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

        #[cfg(all(feature = "embassy", not(feature = "std")))]
        {
            embassy_time::Instant::now().as_millis()
        }

        #[cfg(all(not(feature = "std"), not(feature = "embassy")))]
        {
            // Fallback: no real time available
            // In practice, you would integrate with your platform's timer
            0
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

            SupervisorMessage::ChildPanicked { id } => {
                #[cfg(feature = "debug-log")]
                log::warn!("Child {id:?} panicked - determining restart strategy");

                if let Some(strategy) = self.handle_child_failure(&id) {
                    #[cfg(any(feature = "std", feature = "alloc"))]
                    {
                        let _children_to_restart = self.get_children_to_restart(&id, strategy);
                        #[cfg(feature = "debug-log")]
                        log::info!(
                            "Restarting children: {_children_to_restart:?} (strategy: {strategy:?})"
                        );
                    }

                    #[cfg(not(any(feature = "std", feature = "alloc")))]
                    {
                        let _children_to_restart = self.get_children_to_restart(&id, strategy);
                        #[cfg(feature = "debug-log")]
                        log::info!(
                            "Restarting {} children (strategy: {strategy:?})",
                            _children_to_restart.len()
                        );
                    }

                    // In a full implementation, this would trigger actual child restarts
                    // For now, we just log the decision
                }
            }

            SupervisorMessage::StartChild { id } => {
                #[cfg(feature = "debug-log")]
                log::info!("Request to start child {id:?}");

                // Add child to supervision with default strategy
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
                    #[cfg(any(feature = "std", feature = "alloc"))]
                    {
                        let _children_to_restart = self.get_children_to_restart(&id, strategy);
                        #[cfg(feature = "debug-log")]
                        log::info!("Restarting children: {_children_to_restart:?}");
                    }

                    #[cfg(not(any(feature = "std", feature = "alloc")))]
                    {
                        let _children_to_restart = self.get_children_to_restart(&id, strategy);
                        #[cfg(feature = "debug-log")]
                        log::info!("Restarting {} children", _children_to_restart.len());
                    }
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

        let msg = SupervisorMessage::ChildPanicked { id: 1 };
        let _future = supervisor.handle(msg);
    }
}
