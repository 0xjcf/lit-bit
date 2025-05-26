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
pub trait Actor: Send {
    type Message: Send + 'static;

    fn on_event(&mut self, msg: Self::Message) -> impl core::future::Future<Output = ()> + Send;

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

// Conditional mailbox type aliases (Task 2.1)
#[cfg(not(feature = "std"))]
pub type Inbox<T, const N: usize> = heapless::spsc::Consumer<'static, T, N>;
#[cfg(not(feature = "std"))]
pub type Outbox<T, const N: usize> = heapless::spsc::Producer<'static, T, N>;

#[cfg(feature = "std")]
pub type Inbox<T, const N: usize> = tokio::sync::mpsc::Receiver<T>;
#[cfg(feature = "std")]
pub type Outbox<T, const N: usize> = tokio::sync::mpsc::Sender<T>;

// Platform-specific mailbox creation functions (Tasks 2.2-2.3)
#[cfg(not(feature = "std"))]
#[must_use]
pub fn create_mailbox<T, const N: usize>() -> (Outbox<T, N>, Inbox<T, N>) {
    // For no_std, we need static allocation. In practice, this would be handled
    // by the spawning function that provides the static queue.
    // This is a placeholder that requires external static allocation.
    extern crate alloc;
    use alloc::boxed::Box;
    let queue: &'static mut heapless::spsc::Queue<T, N> =
        Box::leak(Box::new(heapless::spsc::Queue::new()));
    queue.split()
}

#[cfg(feature = "std")]
#[must_use]
pub fn create_mailbox<T, const N: usize>() -> (Outbox<T, N>, Inbox<T, N>) {
    tokio::sync::mpsc::channel(N)
}

// Message processing loop implementation (Task 3.1)
/// Runs an actor's message processing loop.
///
/// # Errors
/// Returns `ActorError` if actor startup, shutdown, or message processing fails.
#[allow(unreachable_code)] // no_std path has infinite loop, cleanup only reachable on std
pub async fn actor_task<A: Actor, const N: usize>(
    mut actor: A,
    mut inbox: Inbox<A::Message, N>,
) -> Result<(), ActorError> {
    // Startup hook
    actor.on_start()?;

    // Main processing loop (Ector pattern)
    loop {
        #[cfg(not(feature = "std"))]
        {
            let msg = loop {
                if let Some(msg) = inbox.dequeue() {
                    break msg;
                }
                // Yield and continue (Embassy style)
                #[cfg(feature = "embassy")]
                embassy_futures::yield_now().await;
                #[cfg(not(feature = "embassy"))]
                {
                    // For no_std without embassy, we need a different yield mechanism
                    // This is a placeholder - in practice you'd use your executor's yield
                }
            };
            actor.on_event(msg).await;
        }

        #[cfg(feature = "std")]
        {
            let Some(msg) = inbox.recv().await else {
                break; // Channel closed
            };
            actor.on_event(msg).await;
        }
    }

    // Cleanup hook
    actor.on_stop()?;
    Ok(())
}

pub mod address;
pub mod backpressure;
pub mod integration;
pub mod spawn;

// Re-export spawn functions for convenience
#[cfg(all(not(feature = "std"), feature = "embassy"))]
pub use spawn::spawn_actor_embassy;
#[cfg(feature = "std")]
pub use spawn::spawn_actor_tokio;

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

        #[allow(clippy::manual_async_fn)] // Need Send bound for thread safety
        fn on_event(&mut self, msg: u32) -> impl core::future::Future<Output = ()> + Send {
            async move {
                self.counter += msg;
            }
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
}
