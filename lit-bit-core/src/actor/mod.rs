//! Minimal Actor trait and supervision primitives for the actor framework.

#![allow(dead_code)]

use core::panic::PanicInfo;
#[cfg(not(feature = "async-tokio"))]
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
/// use core::future::Future;
///
/// struct MyActor {
///     counter: u32,
/// }
///
/// impl Actor for MyActor {
///     type Message = u32;
///     type Future<'a> = impl Future<Output = ()> + 'a where Self: 'a;
///
///     fn handle<'a>(&'a mut self, msg: Self::Message) -> Self::Future<'a> {
///         async move {
///             self.counter += msg;
///             // Async operations can be awaited here
///         }
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
    /// fn handle<'a>(&'a mut self, msg: u32) -> Self::Future<'a> {
    ///     async move {
    ///         self.counter += msg; // Synchronous operation
    ///     }
    /// }
    /// ```
    ///
    /// ### Async handler with I/O
    /// ```rust,no_run
    /// fn handle<'a>(&'a mut self, msg: SensorRequest) -> Self::Future<'a> {
    ///     async move {
    ///         let reading = self.sensor.read().await; // Async I/O
    ///         self.process_reading(reading);
    ///     }
    /// }
    /// ```
    fn handle(&mut self, msg: Self::Message) -> Self::Future<'_>;

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
/// use lit_bit_core::actor::AsyncActor;
///
/// struct HttpActor {
///     client: HttpClient,
/// }
///
/// #[async_trait::async_trait]
/// impl AsyncActor for HttpActor {
///     type Message = HttpRequest;
///
///     async fn handle(&mut self, msg: HttpRequest) {
///         let response = self.client.get(&msg.url).await;
///         // Process response...
///     }
/// }
/// ```
#[cfg(any(feature = "std", feature = "alloc"))]
pub trait AsyncActor: Send {
    /// The message type this actor handles
    type Message: Send + 'static;

    /// Handle a single message asynchronously using ergonomic async fn syntax.
    ///
    /// Note: This method returns a boxed future for ergonomic use when heap allocation
    /// is available. The actual implementation should use async fn syntax when possible.
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
#[cfg(not(feature = "async-tokio"))]
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

/// Creates a mailbox from a statically allocated queue using `StaticCell` (safe alternative).
///
/// This function provides a safe way to create mailboxes from static memory without
/// requiring unsafe code. It uses `StaticCell` to ensure safe one-time initialization.
///
/// For most use cases, prefer the `static_mailbox!` macro which handles the `StaticCell`
/// creation automatically.
///
/// # Arguments
///
/// * `cell` - A `StaticCell` containing an uninitialized heapless queue
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
#[cfg(not(feature = "async-tokio"))]
#[must_use]
pub fn create_mailbox_safe<T, const N: usize>(
    cell: &'static StaticCell<heapless::spsc::Queue<T, N>>,
) -> (Outbox<T, N>, Inbox<T, N>) {
    let queue = cell.init(heapless::spsc::Queue::new());
    queue.split()
}

#[cfg(feature = "async-tokio")]
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

// Message processing loop implementation (Task 3.1)
/// Runs an actor's message processing loop.
///
/// # Errors
/// Returns `ActorError` if actor startup, shutdown, or message processing fails.
#[allow(unreachable_code)] // no_std path has infinite loop, cleanup only reachable on std
#[cfg(not(feature = "async-tokio"))]
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
        actor.handle(msg).await;
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
#[cfg(feature = "async-tokio")]
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
        actor.handle(msg).await;
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
#[cfg(all(not(feature = "async-tokio"), feature = "embassy"))]
pub use spawn::spawn_actor_embassy;
#[cfg(feature = "async-tokio")]
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
}
