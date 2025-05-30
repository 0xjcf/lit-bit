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

/// Supervisor message for communication between supervisor and child actors.
///
/// This message type enables the OTP-style supervision patterns described in the research.
/// Supervisors can receive notifications about child lifecycle events and react accordingly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupervisorMessage<ChildId = u32> {
    /// Child actor has started successfully
    ChildStarted { id: ChildId },
    /// Child actor has stopped gracefully
    ChildStopped { id: ChildId },
    /// Child actor has panicked or failed
    ChildPanicked { id: ChildId },
    /// Request to start a new child actor
    StartChild { id: ChildId },
    /// Request to stop a child actor
    StopChild { id: ChildId },
    /// Request to restart a child actor
    RestartChild { id: ChildId },
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

// Message processing loop implementation (Task 3.1)
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
pub async fn actor_task<A: Actor>(
    mut actor: A,
    mut inbox: Inbox<A::Message>,
) -> Result<(), ActorError> {
    // Startup hook
    let startup_result = actor.on_start();
    #[cfg(feature = "debug-log")]
    if let Err(ref e) = startup_result {
        log::error!("Actor startup failed: {e:?}");
    }
    startup_result?;

    // Main processing loop (std version)
    loop {
        let Some(msg) = inbox.recv().await else {
            break; // Channel closed
        };
        actor.handle(msg).await;
    }

    // Cleanup hook
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
pub async fn batch_actor_task<A: BatchActor>(
    mut actor: A,
    mut inbox: Inbox<A::Message>,
) -> Result<(), ActorError> {
    // Startup hook
    let startup_result = actor.on_start();
    #[cfg(feature = "debug-log")]
    if let Err(ref e) = startup_result {
        log::error!("Batch actor startup failed: {e:?}");
    }
    startup_result?;

    // Main batch processing loop
    let mut batch_buffer = Vec::new();

    loop {
        // Wait for at least one message
        let first_message = match inbox.recv().await {
            Some(msg) => msg,
            None => break, // Channel closed
        };

        batch_buffer.clear();
        batch_buffer.push(first_message);

        // Drain additional messages up to batch limit
        let max_batch = actor.max_batch_size();
        while batch_buffer.len() < max_batch {
            match inbox.try_recv() {
                Ok(msg) => batch_buffer.push(msg),
                Err(_) => break, // No more messages available
            }
        }

        // Process the batch
        actor.handle_batch(&batch_buffer).await;

        // Tokio's cooperative scheduling will automatically yield if needed
        // due to the async/await points above
    }

    // Cleanup hook
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
pub mod spawn;
pub mod supervision; // Task 5.1: Supervision with Async

// Re-export spawn functions for convenience
#[cfg(feature = "async-embassy")]
pub use spawn::spawn_counter_actor_embassy;
#[cfg(all(feature = "async-tokio", not(feature = "async-embassy")))]
pub use spawn::{
    spawn_actor_tokio, spawn_batch_actor_tokio, spawn_supervised_actor_tokio,
    spawn_supervised_batch_actor_tokio,
};

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
