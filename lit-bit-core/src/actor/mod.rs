//! Minimal Actor trait and supervision primitives for the actor framework.

#![allow(dead_code)]

use core::panic::PanicInfo;

/// Error type for actor lifecycle and supervision hooks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActorError {
    StartupFailure,
    ShutdownFailure,
    Panic,
    Custom(&'static str),
}

/// Restart strategy for actor supervision (OTP-inspired).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartStrategy {
    /// Restart only this actor (default)
    OneForOne,
    /// Restart all sibling actors
    OneForAll,
    /// Restart this and all actors started after it
    RestForOne,
}

/// Minimal Actor trait with supervision hooks.
///
/// - `Message`: The event/message type handled by this actor.
/// - `on_event`: Handle a single event (async for compatibility with both std and `no_std` async).
/// - `on_start`: Optional startup hook (default: Ok(())).
/// - `on_stop`: Optional shutdown hook (default: Ok(())).
/// - `on_panic`: Supervision hook for panic handling (default: `OneForOne`).
#[allow(unused_variables)]
#[allow(async_fn_in_trait)]
pub trait Actor {
    type Message: Send + 'static;

    async fn on_event(&mut self, msg: Self::Message);

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
    fn on_panic(&self, info: &PanicInfo) -> RestartStrategy {
        RestartStrategy::OneForOne
    }
}

pub mod address;

#[cfg(test)]
mod tests {
    // use super::*; // Removed unused import
    // TDD: Add tests for Actor trait and future Address/Event integration here.
}
