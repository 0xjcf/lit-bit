//! Minimal Actor trait and supervision primitives for the actor framework.

#![allow(dead_code)]

use core::panic::PanicInfo;
#[cfg(not(feature = "std"))]
use static_cell::StaticCell;

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
///
/// ## Stability Note
///
/// The `on_event` method uses different implementations based on feature flags:
/// - **With `async` feature**: Uses `async-trait` for stable Rust compatibility
/// - **Without `async` feature**: Uses `impl Future` (requires nightly Rust with `async_fn_in_trait` feature)
#[allow(unused_variables)]
#[cfg_attr(not(feature = "async"), allow(async_fn_in_trait))]
pub trait Actor: Send {
    type Message: Send + 'static;

    #[cfg(feature = "async")]
    fn on_event(&mut self, msg: Self::Message) -> futures::future::BoxFuture<'_, ()>;

    #[cfg(not(feature = "async"))]
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
pub type Inbox<T> = tokio::sync::mpsc::Receiver<T>;
#[cfg(feature = "std")]
pub type Outbox<T> = tokio::sync::mpsc::Sender<T>;

// Platform-specific mailbox creation functions (Tasks 2.2-2.3)

/// Creates a static mailbox with safe initialization.
///
/// This macro creates a statically allocated SPSC queue using `StaticCell` and returns
/// the producer and consumer endpoints. It handles initialization safely without any
/// unsafe code and ensures the queue can only be split once.
///
/// # Arguments
///
/// * `$name` - Identifier for the static queue (for debugging/placement control)
/// * `$msg_type` - The message type for the queue
/// * `$capacity` - The queue capacity (const expression)
///
/// # Examples
///
/// ```rust,no_run
/// use lit_bit_core::static_mailbox;
///
/// // Create a mailbox for u32 messages with capacity 16
/// let (producer, consumer) = static_mailbox!(MY_QUEUE: u32, 16);
///
/// // With memory placement attribute
/// let (tx, rx) = static_mailbox!(
///     #[link_section = ".sram2"]
///     FAST_QUEUE: MyMessage, 32
/// );
/// ```
///
/// # Panics
///
/// Panics if called more than once for the same static queue (prevents double-split).
#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! static_mailbox {
    ($(#[$attr:meta])* $name:ident: $msg_type:ty, $capacity:expr) => {{
        use static_cell::StaticCell;

        $(#[$attr])*
        static $name: StaticCell<heapless::spsc::Queue<$msg_type, $capacity>> = StaticCell::new();

        // Initialize the queue and get a 'static reference
        let queue: &'static mut heapless::spsc::Queue<$msg_type, $capacity> =
            $name.init(heapless::spsc::Queue::new());

        // Split the queue into producer and consumer
        queue.split()
    }};

    // Variant without attributes
    ($name:ident: $msg_type:ty, $capacity:expr) => {{
        use static_cell::StaticCell;

        static $name: StaticCell<heapless::spsc::Queue<$msg_type, $capacity>> = StaticCell::new();

        // Initialize the queue and get a 'static reference
        let queue: &'static mut heapless::spsc::Queue<$msg_type, $capacity> =
            $name.init(heapless::spsc::Queue::new());

        // Split the queue into producer and consumer
        queue.split()
    }};
}

/// Creates a mailbox from a statically allocated queue using StaticCell (safe alternative).
///
/// This function provides a safe way to create mailboxes from static memory without
/// requiring unsafe code. It uses `StaticCell` to ensure safe one-time initialization.
///
/// For most use cases, prefer the `static_mailbox!` macro which handles the StaticCell
/// creation automatically.
///
/// # Arguments
///
/// * `cell` - A StaticCell containing an uninitialized heapless queue
///
/// # Examples
///
/// ```rust,no_run
/// use heapless::spsc::Queue;
/// use static_cell::StaticCell;
/// use lit_bit_core::actor::create_mailbox_safe;
///
/// static QUEUE_CELL: StaticCell<Queue<u32, 16>> = StaticCell::new();
///
/// let (outbox, inbox) = create_mailbox_safe(&QUEUE_CELL);
/// ```
#[cfg(not(feature = "std"))]
#[must_use]
pub fn create_mailbox_safe<T, const N: usize>(
    cell: &'static StaticCell<heapless::spsc::Queue<T, N>>,
) -> (Outbox<T, N>, Inbox<T, N>) {
    let queue = cell.init(heapless::spsc::Queue::new());
    queue.split()
}

#[cfg(feature = "std")]
#[must_use]
pub fn create_mailbox<T>(capacity: usize) -> (Outbox<T>, Inbox<T>) {
    tokio::sync::mpsc::channel(capacity)
}

/// Yield mechanism for `no_std` environments without Embassy.
///
/// This provides a default yield implementation that allows the executor to schedule
/// other tasks when the message queue is empty. The implementation returns `Poll::Pending`
/// once before completing, which gives the executor an opportunity to run other tasks.
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
#[cfg(all(not(feature = "std"), not(feature = "embassy")))]
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

// Message processing loop implementation (Task 3.1)
/// Runs an actor's message processing loop.
///
/// # Errors
/// Returns `ActorError` if actor startup, shutdown, or message processing fails.
#[allow(unreachable_code)] // no_std path has infinite loop, cleanup only reachable on std
#[cfg(not(feature = "std"))]
pub async fn actor_task<A: Actor, const N: usize>(
    mut actor: A,
    mut inbox: Inbox<A::Message, N>,
) -> Result<(), ActorError> {
    // Startup hook
    actor.on_start()?;

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
        actor.on_event(msg).await;
    }

    // Cleanup hook (unreachable in no_std)
    #[allow(unreachable_code)]
    {
        actor.on_stop()?;
        Ok(())
    }
}

/// Runs an actor's message processing loop (std version).
///
/// # Errors
/// Returns `ActorError` if actor startup, shutdown, or message processing fails.
#[cfg(feature = "std")]
pub async fn actor_task<A: Actor>(
    mut actor: A,
    mut inbox: Inbox<A::Message>,
) -> Result<(), ActorError> {
    // Startup hook
    actor.on_start()?;

    // Main processing loop (std version)
    loop {
        let Some(msg) = inbox.recv().await else {
            break; // Channel closed
        };
        actor.on_event(msg).await;
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

        #[cfg(feature = "async")]
        fn on_event(&mut self, msg: u32) -> futures::future::BoxFuture<'_, ()> {
            Box::pin(async move {
                self.counter += msg;
            })
        }

        #[cfg(not(feature = "async"))]
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
}
