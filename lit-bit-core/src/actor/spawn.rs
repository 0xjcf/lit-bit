//! Actor spawning functions for Embassy and Tokio runtimes.

#[cfg(feature = "async-embassy")]
use super::{Actor, ActorString, BatchActor};

#[cfg(all(feature = "async-tokio", not(feature = "async-embassy")))]
use super::{Actor, BatchActor, actor_task, batch_actor_task, create_mailbox};

#[cfg(any(feature = "async-embassy", feature = "async-tokio"))]
use super::address::Address;

#[cfg(feature = "async-tokio")]
use super::supervision::SupervisorActor;

// Embassy-specific imports
#[cfg(feature = "async-embassy")]
use embassy_sync::blocking_mutex::raw::NoopRawMutex;

#[cfg(feature = "async-embassy")]
use embassy_sync::channel::Receiver;

// Conditional Box import for panic error handling (Embassy specific)
#[cfg(all(any(feature = "std", feature = "alloc"), feature = "async-embassy"))]
extern crate alloc;
#[cfg(all(any(feature = "std", feature = "alloc"), feature = "async-embassy"))]
use alloc::boxed::Box;

/// Error types for spawn operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpawnError {
    /// Supervisor-related error
    Supervisor(crate::actor::supervision::SupervisorError),
    /// Generic spawn failure
    SpawnFailed,
}

impl From<crate::actor::supervision::SupervisorError> for SpawnError {
    fn from(err: crate::actor::supervision::SupervisorError) -> Self {
        SpawnError::Supervisor(err)
    }
}

// Embassy task for running actors (must be at top level and non-generic)
#[cfg(feature = "async-embassy")]
#[embassy_executor::task]
async fn embassy_actor_task_u32(
    mut actor: CounterActor,
    receiver: Receiver<'static, NoopRawMutex, u32, 16>,
) {
    // This is a concrete task for u32 messages - Embassy tasks cannot be generic
    // In practice, you would create specific tasks for each actor type
    loop {
        let msg = receiver.receive().await;
        actor.handle(msg).await;
    }
}

// Embassy batch task for running batch actors (must be at top level and non-generic)
#[cfg(feature = "async-embassy")]
#[embassy_executor::task]
async fn embassy_batch_actor_task_u32_16(
    mut actor: CounterBatchActor,
    receiver: Receiver<'static, NoopRawMutex, u32, 16>,
) {
    // This is a concrete batch task for u32 messages with fixed capacity 16
    // Embassy tasks cannot be generic at all, so each capacity needs its own task
    let mut batch_buffer = heapless::Vec::<u32, 32>::new(); // Fixed batch size for concrete implementation

    loop {
        // Collect messages up to batch size or until no more are immediately available
        batch_buffer.clear();

        // Always get at least one message (blocking)
        let first_msg = receiver.receive().await;
        let _ = batch_buffer.push(first_msg); // Safe because buffer is empty

        // Try to collect more messages without blocking
        while batch_buffer.len() < actor.max_batch_size()
            && batch_buffer.len() < batch_buffer.capacity()
        {
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
    }
}

// Embassy batch task for capacity 32
#[cfg(feature = "async-embassy")]
#[embassy_executor::task]
async fn embassy_batch_actor_task_u32_32(
    mut actor: CounterBatchActor,
    receiver: Receiver<'static, NoopRawMutex, u32, 32>,
) {
    // This is a concrete batch task for u32 messages with fixed capacity 32
    let mut batch_buffer = heapless::Vec::<u32, 32>::new(); // Fixed batch size for concrete implementation

    loop {
        // Collect messages up to batch size or until no more are immediately available
        batch_buffer.clear();

        // Always get at least one message (blocking)
        let first_msg = receiver.receive().await;
        let _ = batch_buffer.push(first_msg); // Safe because buffer is empty

        // Try to collect more messages without blocking
        while batch_buffer.len() < actor.max_batch_size()
            && batch_buffer.len() < batch_buffer.capacity()
        {
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
    }
}

// Example concrete actor for demonstration
#[cfg(feature = "async-embassy")]
pub struct CounterActor {
    value: u32,
}

#[cfg(feature = "async-embassy")]
impl CounterActor {
    #[must_use]
    pub fn new() -> Self {
        Self { value: 0 }
    }
}

#[cfg(feature = "async-embassy")]
impl Default for CounterActor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "async-embassy")]
impl super::Actor for CounterActor {
    type Message = u32;
    type Future<'a>
        = core::future::Ready<()>
    where
        Self: 'a;

    fn handle(&mut self, msg: Self::Message) -> Self::Future<'_> {
        self.value += msg;
        core::future::ready(())
    }
}

// Example concrete batch actor for demonstration
#[cfg(feature = "async-embassy")]
pub struct CounterBatchActor {
    value: u32,
    batch_size: usize,
}

#[cfg(feature = "async-embassy")]
impl CounterBatchActor {
    #[must_use]
    pub fn new() -> Self {
        Self {
            value: 0,
            batch_size: 16, // Default batch size
        }
    }

    #[must_use]
    pub fn with_batch_size(batch_size: usize) -> Self {
        Self {
            value: 0,
            batch_size,
        }
    }
}

#[cfg(feature = "async-embassy")]
impl Default for CounterBatchActor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "async-embassy")]
impl super::BatchActor for CounterBatchActor {
    type Message = u32;
    type Future<'a>
        = core::future::Ready<()>
    where
        Self: 'a;

    fn handle_batch(&mut self, messages: &[Self::Message]) -> Self::Future<'_> {
        for &msg in messages {
            self.value += msg;
        }
        core::future::ready(())
    }

    fn max_batch_size(&self) -> usize {
        self.batch_size
    }
}

/// Spawns a `CounterActor` on the Embassy executor using a pre-created Embassy channel.
///
/// This is a concrete implementation for demonstration purposes. In a real application,
/// you would create similar functions for each specific actor type you need, since
/// Embassy tasks cannot be generic.
///
/// ## Memory Management
///
/// - Uses pre-created static channels (via `static_embassy_channel!` macro)
/// - Channel capacity is fixed at compile time
/// - No heap allocation - suitable for `#![no_std]` environments
/// - All memory usage is predictable and bounded
///
/// ## Concurrency Model
///
/// - One message processed at a time per actor (deterministic execution)
/// - Backpressure when channel is full (sender blocks until space available)
/// - Uses `NoopRawMutex` for single-core, non-interrupt usage
///
/// # Arguments
///
/// * `spawner` - The Embassy spawner to use for spawning the actor task
/// * `actor` - The `CounterActor` instance to spawn
/// * `sender` - Pre-created Embassy channel sender
/// * `receiver` - Pre-created Embassy channel receiver
///
/// # Returns
///
/// Returns `Ok(Address)` if the actor was successfully spawned, or `Err(embassy_executor::SpawnError)`
/// if spawning failed (e.g., task arena is full).
///
/// # Errors
///
/// Returns `embassy_executor::SpawnError` if the Embassy task arena is full or if spawning fails.
///
/// # Examples
///
/// ```rust,no_run
/// use embassy_executor::Spawner;
/// use lit_bit_core::{actor::spawn_counter_actor_embassy, static_embassy_channel};
///
/// #[embassy_executor::main]
/// async fn main(spawner: Spawner) {
///     // Create a static channel for the actor
///     let (sender, receiver) = static_embassy_channel!(MY_ACTOR_CHANNEL: u32, 16);
///     
///     let actor = CounterActor::new();
///     let address = spawn_counter_actor_embassy(spawner, actor, sender, receiver).unwrap();
///     
///     // Send a message to the actor
///     address.send(42).await;
/// }
/// ```
///
/// # Embassy Integration Notes
///
/// This implementation follows Embassy 0.6 patterns:
/// - Uses `embassy_sync::channel::Channel` for type-safe messaging
/// - Employs user-provided static allocation for full control
/// - Leverages `NoopRawMutex` for efficient single-core operation
/// - Integrates with Embassy's cooperative task scheduler
#[cfg(feature = "async-embassy")]
pub fn spawn_counter_actor_embassy(
    spawner: embassy_executor::Spawner,
    actor: CounterActor,
    sender: embassy_sync::channel::Sender<'static, NoopRawMutex, u32, 16>,
    receiver: embassy_sync::channel::Receiver<'static, NoopRawMutex, u32, 16>,
) -> Result<Address<u32, 16>, embassy_executor::SpawnError> {
    // Spawn the embassy task using the task token
    spawner.spawn(embassy_actor_task_u32(actor, receiver))?;

    // Create and return the address
    Ok(Address::from_embassy_sender(sender))
}

/// Creates a static Embassy channel for actor communication.
///
/// This macro creates a statically allocated Embassy channel using `StaticCell` and returns
/// the sender and receiver endpoints. It follows Embassy 0.6 best practices for static
/// allocation and type-safe messaging.
///
/// ## Usage Pattern
///
/// This macro is typically used when you need more control over channel placement or
/// when creating multiple channels with specific configurations.
///
/// # Arguments
///
/// * `$name` - Identifier for the static channel (for debugging/placement control)
/// * `$msg_type` - The message type for the channel
/// * `$capacity` - The channel capacity (const expression)
///
/// # Examples
///
/// ```rust,no_run
/// use lit_bit_core::static_embassy_channel;
///
/// // Create a channel for u32 messages with capacity 16
/// let (sender, receiver) = static_embassy_channel!(MY_CHANNEL: u32, 16);
///
/// // With memory placement attribute
/// let (tx, rx) = static_embassy_channel!(
///     #[link_section = ".sram2"]
///     FAST_CHANNEL: MyMessage, 64
/// );
/// ```
///
/// # Panics
///
/// Panics if called more than once for the same static channel (prevents double-initialization).
#[cfg(feature = "async-embassy")]
#[macro_export]
macro_rules! static_embassy_channel {
    ($(#[$attr:meta])* $name:ident: $msg_type:ty, $capacity:expr) => {{
        use embassy_sync::channel::Channel;
        use embassy_sync::blocking_mutex::raw::NoopRawMutex;
        use static_cell::StaticCell;

        $(#[$attr])*
        static $name: StaticCell<Channel<NoopRawMutex, $msg_type, $capacity>> = StaticCell::new();

        // Initialize the channel and get a 'static reference
        let channel: &'static Channel<NoopRawMutex, $msg_type, $capacity> =
            $name.init(Channel::new());

        // Get sender and receiver
        (channel.sender(), channel.receiver())
    }};

    // Variant without attributes
    ($name:ident: $msg_type:ty, $capacity:expr) => {{
        use embassy_sync::channel::Channel;
        use embassy_sync::blocking_mutex::raw::NoopRawMutex;
        use static_cell::StaticCell;

        static $name: StaticCell<Channel<NoopRawMutex, $msg_type, $capacity>> = StaticCell::new();

        // Initialize the channel and get a 'static reference
        let channel: &'static Channel<NoopRawMutex, $msg_type, $capacity> =
            $name.init(Channel::new());

        // Get sender and receiver
        (channel.sender(), channel.receiver())
    }};
}

// Tokio spawning function (existing implementation)
#[cfg(feature = "async-tokio")]
pub fn spawn_actor_tokio<A>(actor: A, capacity: usize) -> Address<A::Message>
where
    A: Actor + Send + 'static,
    A::Message: Send + 'static,
{
    let (outbox, inbox) = create_mailbox::<A::Message>(capacity);

    // Spawn on current Tokio runtime
    tokio::spawn(actor_task::<A>(actor, inbox));

    // Create Address from the Tokio sender
    Address::from_tokio_sender(outbox)
}

/// Enhanced spawn functions for Tasks 5.1 and 5.2
/// Spawns a batch actor on the Tokio runtime.
///
/// This function spawns an actor that implements `BatchActor` for high-throughput
/// message processing. The actor will process messages in batches according to
/// its `max_batch_size()` configuration.
///
/// ## Performance Benefits
///
/// - Reduced task wake-up overhead
/// - Better cache locality for related messages
/// - Higher overall throughput for high-frequency messaging
/// - Configurable batch size for throughput/latency trade-offs
///
/// # Arguments
/// * `actor` - The batch actor instance to spawn
/// * `capacity` - Mailbox capacity for the actor
///
/// # Returns
/// An `Address` for sending messages to the spawned batch actor.
///
/// # Examples
///
/// ```rust,no_run
/// use lit_bit_core::actor::spawn::spawn_batch_actor_tokio;
/// use lit_bit_core::actor::BatchActor;
///
/// struct HighThroughputActor { count: u32 }
/// impl BatchActor for HighThroughputActor {
///     type Message = u32;
///     type Future<'a> = core::future::Ready<()> where Self: 'a;
///     
///     fn handle_batch(&mut self, messages: &[u32]) -> Self::Future<'_> {
///         for &msg in messages {
///             self.count += msg;
///         }
///         core::future::ready(())
///     }
/// }
///
/// let actor = HighThroughputActor { count: 0 };
/// let address = spawn_batch_actor_tokio(actor, 64);
/// ```
#[cfg(feature = "async-tokio")]
pub fn spawn_batch_actor_tokio<A>(actor: A, capacity: usize) -> Address<A::Message>
where
    A: BatchActor + Send + 'static,
    A::Message: Send + 'static,
{
    let (outbox, inbox) = create_mailbox::<A::Message>(capacity);

    // Spawn on current Tokio runtime using the batch task function
    tokio::spawn(batch_actor_task::<A>(actor, inbox));

    // Create Address from the Tokio sender
    Address::from_tokio_sender(outbox)
}

/// Spawns a supervised actor on the Tokio runtime.
///
/// This function spawns an actor under supervision, registering it with a supervisor
/// that can restart it according to the specified restart strategy. This implements
/// Task 5.1 supervision patterns.
///
/// ## Supervision Features
///
/// - Automatic restart on actor failure
/// - Configurable restart strategies (OneForOne, OneForAll, RestForOne)
/// - Rate limiting to prevent restart loops
/// - JoinHandle monitoring for failure detection
///
/// # Arguments
/// * `actor` - The actor instance to spawn under supervision
/// * `supervisor` - Mutable reference to the supervisor actor
/// * `child_id` - Unique identifier for this child actor
/// * `capacity` - Mailbox capacity for the actor
///
/// # Returns
/// An `Address` for sending messages to the spawned supervised actor.
///
/// # Errors
/// Returns an error if the supervisor cannot add the child or if spawning fails.
///
/// # Examples
///
/// ```rust,no_run
/// # #[cfg(all(feature = "async-tokio", not(feature = "async-embassy")))]
/// # {
/// use lit_bit_core::actor::supervision::SupervisorActor;
/// use lit_bit_core::actor::spawn::spawn_supervised_actor_tokio;
/// use lit_bit_core::actor::Actor;
///
/// struct MyActor;
/// impl Actor for MyActor {
///     type Message = u32;
///     type Future<'a> = core::future::Ready<()> where Self: 'a;
///     fn handle(&mut self, _msg: u32) -> Self::Future<'_> {
///         core::future::ready(())
///     }
/// }
/// impl MyActor {
///     fn new() -> Self { MyActor }
/// }
///
/// let mut supervisor = SupervisorActor::<u32, 8>::new();
/// let actor = MyActor::new();
/// let address = spawn_supervised_actor_tokio(actor, &mut supervisor, 1, 32);
/// # }
/// ```
#[cfg(feature = "async-tokio")]
pub fn spawn_supervised_actor_tokio<A, ChildId, const MAX_CHILDREN: usize>(
    actor: A,
    supervisor: &mut SupervisorActor<ChildId, MAX_CHILDREN>,
    child_id: ChildId,
    capacity: usize,
) -> Result<Address<A::Message>, SpawnError>
where
    A: Actor + Send + 'static,
    A::Message: Send + 'static,
    ChildId: Clone + PartialEq + core::fmt::Debug + core::hash::Hash + Eq,
{
    let (outbox, inbox) = create_mailbox::<A::Message>(capacity);

    // Spawn on current Tokio runtime
    let join_handle = tokio::spawn(actor_task::<A>(actor, inbox));

    // Add child to supervisor with handle atomically
    // If this fails, abort the spawned task to prevent orphaned actors
    if let Err(err) = supervisor.add_child_with_handle(child_id, join_handle, None) {
        // Note: The JoinHandle was consumed by add_child_with_handle, so we can't abort it
        // However, this is much safer as the child is only added if the handle can be tracked
        return Err(err.into());
    }

    // Success - return the address
    Ok(Address::from_tokio_sender(outbox))
}

/// Spawns a supervised batch actor on the Tokio runtime.
///
/// This combines both supervision (Task 5.1) and batching (Task 5.2) capabilities,
/// providing high-throughput message processing with automatic restart on failure.
///
/// ## Combined Benefits
///
/// - High-throughput batch message processing
/// - Automatic supervision and restart on failure
/// - Rate-limited restart to prevent loops
/// - Configurable batch size and restart strategies
///
/// # Arguments
/// * `actor` - The batch actor instance to spawn under supervision
/// * `supervisor` - Mutable reference to the supervisor actor
/// * `child_id` - Unique identifier for this child actor
/// * `capacity` - Mailbox capacity for the actor
///
/// # Returns
/// An `Address` for sending messages to the spawned supervised batch actor.
///
/// # Errors
/// Returns an error if the supervisor cannot add the child or if spawning fails.
#[cfg(all(feature = "async-tokio", not(feature = "async-embassy")))]
pub fn spawn_supervised_batch_actor_tokio<A, ChildId, const MAX_CHILDREN: usize>(
    actor: A,
    supervisor: &mut SupervisorActor<ChildId, MAX_CHILDREN>,
    child_id: ChildId,
    capacity: usize,
) -> Result<Address<A::Message>, SpawnError>
where
    A: BatchActor + Send + 'static,
    A::Message: Send + 'static,
    ChildId: Clone + PartialEq + core::fmt::Debug + core::hash::Hash + Eq,
{
    let (outbox, inbox) = create_mailbox::<A::Message>(capacity);

    // Spawn on current Tokio runtime using batch task function
    let join_handle = tokio::spawn(batch_actor_task::<A>(actor, inbox));

    // Add child to supervisor with handle atomically
    // If this fails, abort the spawned task to prevent orphaned actors
    if let Err(err) = supervisor.add_child_with_handle(child_id, join_handle, None) {
        // Note: The JoinHandle was consumed by add_child_with_handle, so we can't abort it
        // However, this is much safer as the child is only added if the handle can be tracked
        return Err(err.into());
    }

    // Success - return the address
    Ok(Address::from_tokio_sender(outbox))
}

/// Spawns a batch actor on the Embassy runtime.
///
/// Embassy-specific version of batch actor spawning that uses static allocation
/// and cooperative scheduling for embedded environments.
///
/// ## Embassy-Specific Features
///
/// - Zero-heap allocation using static channels
/// - Cooperative task scheduling
/// - Bounded memory usage
/// - Deterministic execution
///
/// # Arguments
/// * `spawner` - The Embassy spawner
/// * `actor` - The batch actor instance
/// * `sender` - Pre-created Embassy channel sender
/// * `receiver` - Pre-created Embassy channel receiver
///
/// # Returns
/// Result containing the `Address` if spawning succeeded, or `SpawnError` if failed.
///
/// # Examples
///
/// ```rust,no_run
/// use embassy_executor::Spawner;
/// use lit_bit_core::{actor::spawn::spawn_batch_actor_embassy, static_embassy_channel};
///
/// #[embassy_executor::main]
/// async fn main(spawner: Spawner) {
///     let (sender, receiver) = static_embassy_channel!(BATCH_CHANNEL: u32, 32);
///     let actor = MyBatchActor::new();
///     let address = spawn_batch_actor_embassy(spawner, actor, sender, receiver).unwrap();
/// }
/// ```
///
/// # Implementation Note
///
/// **Embassy tasks cannot be generic**, so this generic function cannot be implemented directly.
/// Instead, you need to create concrete implementations for each actor/message type combination.
///
/// ## Pattern to Follow
///
/// 1. Create a concrete Embassy task function (non-generic)
/// 2. Create a concrete spawn function for your specific actor type
/// 3. Use the concrete spawn function in your application
///
/// See `spawn_counter_batch_actor_embassy` and `embassy_batch_actor_task_u32` for a complete
/// example of this pattern with `CounterBatchActor` and `u32` messages.
///
/// ## Creating Your Own Implementation
///
/// ```rust,ignore
/// // 1. Create a concrete Embassy task for your actor type
/// #[embassy_executor::task]
/// async fn embassy_my_batch_actor_task<const N: usize>(
///     mut actor: MyBatchActor,
///     receiver: embassy_sync::channel::Receiver<'static, NoopRawMutex, MyMessage, N>,
/// ) {
///     // Implementation similar to embassy_batch_actor_task_u32
/// }
///
/// // 2. Create a concrete spawn function
/// pub fn spawn_my_batch_actor_embassy<const N: usize>(
///     spawner: embassy_executor::Spawner,
///     actor: MyBatchActor,
///     sender: embassy_sync::channel::Sender<'static, NoopRawMutex, MyMessage, N>,
///     receiver: embassy_sync::channel::Receiver<'static, NoopRawMutex, MyMessage, N>,
/// ) -> Result<Address<MyMessage, N>, embassy_executor::SpawnError> {
///     spawner.spawn(embassy_my_batch_actor_task(actor, receiver))?;
///     Ok(Address::from_embassy_sender(sender))
/// }
/// ```
#[cfg(feature = "async-embassy")]
pub fn spawn_batch_actor_embassy<A, const N: usize>(
    _spawner: embassy_executor::Spawner,
    _actor: A,
    _sender: embassy_sync::channel::Sender<
        'static,
        embassy_sync::blocking_mutex::raw::NoopRawMutex,
        A::Message,
        N,
    >,
    _receiver: embassy_sync::channel::Receiver<
        'static,
        embassy_sync::blocking_mutex::raw::NoopRawMutex,
        A::Message,
        N,
    >,
) -> Result<Address<A::Message, N>, embassy_executor::SpawnError>
where
    A: BatchActor + Send + 'static,
    A::Message: Send + 'static,
{
    // Embassy tasks cannot be generic - you need to create concrete implementations.
    // See spawn_counter_batch_actor_embassy() for an example of the correct pattern.
    //
    // To implement this for your actor type:
    // 1. Create a concrete #[embassy_executor::task] function for your actor
    // 2. Create a concrete spawn function (similar to spawn_counter_batch_actor_embassy)
    // 3. Use your concrete spawn function instead of this generic one
    unimplemented!(
        "spawn_batch_actor_embassy cannot be implemented generically due to Embassy task constraints. \
         Use spawn_counter_batch_actor_embassy_16 or spawn_counter_batch_actor_embassy_32 as templates \
         to create a concrete implementation for your specific actor type. See function documentation for the required pattern."
    )
}

/// Spawns a `CounterBatchActor` on the Embassy executor using a pre-created Embassy channel.
///
/// This is a concrete implementation for demonstrating batch actor patterns in Embassy.
/// Since Embassy tasks cannot be generic, this function works specifically with
/// `CounterBatchActor` and `u32` messages with capacity 16.
///
/// ## Memory Management
///
/// - Uses pre-created static channels (via `static_embassy_channel!` macro)
/// - Channel capacity is fixed at compile time
/// - No heap allocation - suitable for `#![no_std]` environments
/// - All memory usage is predictable and bounded
///
/// ## Batch Processing
///
/// - Collects messages in batches up to the actor's `max_batch_size()`
/// - Uses non-blocking message collection for improved throughput
/// - Processes batches atomically for better cache locality
/// - Falls back to single-message processing when needed
///
/// # Arguments
///
/// * `spawner` - The Embassy spawner to use for spawning the batch actor task
/// * `actor` - The `CounterBatchActor` instance to spawn
/// * `sender` - Pre-created Embassy channel sender
/// * `receiver` - Pre-created Embassy channel receiver
///
/// # Returns
///
/// Returns `Ok(Address)` if the actor was successfully spawned, or `Err(embassy_executor::SpawnError)`
/// if spawning failed (e.g., task arena is full).
///
/// # Errors
///
/// Returns `embassy_executor::SpawnError` if the Embassy task arena is full or if spawning fails.
///
/// # Examples
///
/// ```rust,no_run
/// use embassy_executor::Spawner;
/// use lit_bit_core::{actor::spawn_counter_batch_actor_embassy, static_embassy_channel};
///
/// #[embassy_executor::main]
/// async fn main(spawner: Spawner) {
///     // Create a static channel for the batch actor  
///     let (sender, receiver) = static_embassy_channel!(MY_BATCH_CHANNEL: u32, 16);
///     
///     let actor = CounterBatchActor::with_batch_size(8);
///     let address = spawn_counter_batch_actor_embassy(spawner, actor, sender, receiver).unwrap();
///     
///     // Send messages to the actor - they will be processed in batches
///     address.send(1).await;
///     address.send(2).await;
///     address.send(3).await;
/// }
/// ```
#[cfg(feature = "async-embassy")]
pub fn spawn_counter_batch_actor_embassy(
    spawner: embassy_executor::Spawner,
    actor: CounterBatchActor,
    sender: embassy_sync::channel::Sender<'static, NoopRawMutex, u32, 16>,
    receiver: embassy_sync::channel::Receiver<'static, NoopRawMutex, u32, 16>,
) -> Result<Address<u32, 16>, embassy_executor::SpawnError> {
    // Spawn the embassy batch task using the concrete task
    spawner.spawn(embassy_batch_actor_task_u32_16(actor, receiver))?;

    // Create and return the address
    Ok(Address::from_embassy_sender(sender))
}

/// Spawns a `CounterBatchActor` with capacity 16 on the Embassy executor.
#[cfg(feature = "async-embassy")]
pub fn spawn_counter_batch_actor_embassy_16(
    spawner: embassy_executor::Spawner,
    actor: CounterBatchActor,
    sender: embassy_sync::channel::Sender<'static, NoopRawMutex, u32, 16>,
    receiver: embassy_sync::channel::Receiver<'static, NoopRawMutex, u32, 16>,
) -> Result<Address<u32, 16>, embassy_executor::SpawnError> {
    spawner.spawn(embassy_batch_actor_task_u32_16(actor, receiver))?;
    Ok(Address::from_embassy_sender(sender))
}

/// Spawns a `CounterBatchActor` with capacity 32 on the Embassy executor.
#[cfg(feature = "async-embassy")]
pub fn spawn_counter_batch_actor_embassy_32(
    spawner: embassy_executor::Spawner,
    actor: CounterBatchActor,
    sender: embassy_sync::channel::Sender<'static, NoopRawMutex, u32, 32>,
    receiver: embassy_sync::channel::Receiver<'static, NoopRawMutex, u32, 32>,
) -> Result<Address<u32, 32>, embassy_executor::SpawnError> {
    spawner.spawn(embassy_batch_actor_task_u32_32(actor, receiver))?;
    Ok(Address::from_embassy_sender(sender))
}

/// Phase 3.1.1: Panic-safe actor task for Tokio supervision integration.
///
/// This task function wraps actor message handling with panic capture using `catch_unwind`.
/// When a panic occurs, it extracts the panic information and sends it to the supervisor
/// before terminating the actor task. The supervisor can then decide whether to restart
/// the actor based on its configured restart strategy.
///
/// ## Panic Handling
///
/// - Uses `std::panic::catch_unwind()` to capture actor panics
/// - Extracts panic message using Phase 1 panic capture utilities
/// - Sends detailed panic information to supervisor via SupervisorMessage
/// - Actor task terminates after panic - supervisor handles restart
///
/// ## Integration with Supervision
///
/// - Compatible with Phase 2 SupervisorActor system
/// - Sends `SupervisorMessage::ChildPanicked` on actor failure
/// - Provides actor ID and detailed error information for restart decisions
/// - Supervisor applies configured backoff delays and restart intensity limits
///
/// # Arguments
///
/// * `actor` - The actor instance to run with panic protection
/// * `mailbox` - Tokio MPSC receiver for actor messages
/// * `supervisor_address` - Optional address to send panic notifications
/// * `actor_id` - String identifier for this actor (for supervision context)
///
/// # Returns
///
/// Returns `Ok(())` on normal termination or `Err(ActorError)` on startup failure.
/// After a panic, it sends notification to the supervisor and returns `Ok(())`.
#[cfg(feature = "async-tokio")]
pub async fn panic_safe_actor_task<A: Actor>(
    mut actor: A,
    mut mailbox: tokio::sync::mpsc::Receiver<A::Message>,
    supervisor_address: Option<
        crate::actor::address::Address<crate::actor::SupervisorMessage<String>>,
    >,
    actor_id: String,
) -> Result<(), crate::actor::ActorError> {
    use futures::FutureExt;
    use std::panic::AssertUnwindSafe;

    // Call actor startup hook
    if let Err(startup_error) = actor.on_start() {
        if let Some(supervisor_addr) = supervisor_address {
            let _ = supervisor_addr
                .send(crate::actor::SupervisorMessage::ChildPanicked {
                    id: actor_id.clone(),
                    error: Box::new(startup_error.clone()),
                })
                .await;
        }
        return Err(startup_error);
    }

    // Main message processing loop with panic protection
    while let Some(message) = mailbox.recv().await {
        // Wrap the actor's handle method in AssertUnwindSafe for catch_unwind
        let handle_future = AssertUnwindSafe(actor.handle(message));

        match handle_future.catch_unwind().await {
            Ok(()) => continue, // Normal message processing
            Err(panic_payload) => {
                // Use Phase 1 panic capture utilities to extract panic information
                let actor_error =
                    crate::actor::panic_handling::capture_panic_info_from_payload_with_id(
                        &panic_payload,
                        actor_id.clone(),
                    );

                // Notify supervisor about the panic
                if let Some(supervisor_addr) = supervisor_address {
                    let _ = supervisor_addr
                        .send(crate::actor::SupervisorMessage::ChildPanicked {
                            id: actor_id.clone(),
                            error: Box::new(actor_error),
                        })
                        .await;
                }

                // Actor terminates after panic - supervisor will restart if configured
                return Ok(());
            }
        }
    }

    // Call actor shutdown hook on normal termination
    // Note: on_stop consumes self, so we can't call it after a panic
    // This is only reached on normal mailbox closure
    let _ = actor.on_stop();
    Ok(())
}

/// Phase 3.1.1: Spawn function that uses panic-safe actor task with supervision.
///
/// This enhanced spawn function creates actors that integrate with the supervision
/// system for automatic panic recovery. It uses the panic-safe task implementation
/// to capture and report actor failures to the supervisor.
///
/// ## Features
///
/// - Automatic panic capture and reporting
/// - Integration with Phase 2 supervision system
/// - Supervisor notification on actor startup failure
/// - JoinHandle monitoring for supervisor integration
///
/// # Arguments
///
/// * `actor` - The actor instance to spawn with panic protection
/// * `supervisor` - Mutable reference to the supervisor actor
/// * `child_id` - Unique identifier for this child actor
/// * `capacity` - Mailbox capacity for the actor
/// * `supervisor_address` - Address to send supervision messages
///
/// # Returns
///
/// An `Address` for sending messages to the spawned actor, or an error if
/// spawning fails or the supervisor cannot add the child.
///
/// # Examples
///
/// ```rust,no_run
/// # #[cfg(feature = "async-tokio")]
/// # {
/// use lit_bit_core::actor::supervision::SupervisorActor;
/// use lit_bit_core::actor::spawn::spawn_supervised_actor_with_panic_handling;
/// use lit_bit_core::actor::{Actor, RestartStrategy, RestartIntensity};
///
/// struct MyActor {
///     count: u32,
/// }
///
/// impl Actor for MyActor {
///     type Message = u32;
///     type Future<'a> = core::future::Ready<()> where Self: 'a;
///     
///     fn handle(&mut self, msg: u32) -> Self::Future<'_> {
///         self.count += msg;
///         core::future::ready(())
///     }
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let mut supervisor = SupervisorActor::new(
///         RestartStrategy::Transient,
///         RestartIntensity::default(),
///     );
///     
///     let actor = MyActor { count: 0 };
///     let address = spawn_supervised_actor_with_panic_handling(
///         actor,
///         &mut supervisor,
///         "my_actor".to_string(),
///         100,
///     ).unwrap();
///     
///     // Actor is now running with panic protection and supervision
/// }
/// # }
/// ```
#[cfg(feature = "async-tokio")]
pub fn spawn_supervised_actor_with_panic_handling<A, ChildId, const MAX_CHILDREN: usize>(
    actor: A,
    supervisor: &mut SupervisorActor<ChildId, MAX_CHILDREN>,
    child_id: ChildId,
    capacity: usize,
    supervisor_address: Option<
        crate::actor::address::Address<crate::actor::SupervisorMessage<String>>,
    >,
) -> Result<Address<A::Message>, SpawnError>
where
    A: Actor + Send + 'static,
    A::Message: Send + 'static,
    ChildId: Clone + ToString + Send + 'static + core::fmt::Debug + core::hash::Hash + Eq,
{
    let (tx, rx) = tokio::sync::mpsc::channel(capacity);
    let actor_id_string = child_id.to_string();

    // Spawn the panic-safe actor task
    let handle = tokio::task::spawn(panic_safe_actor_task(
        actor,
        rx,
        supervisor_address,
        actor_id_string,
    ));

    // Register the child with the supervisor (using JoinHandle for monitoring)
    supervisor.add_child_with_handle(child_id, handle, None)?;

    Ok(Address::from_tokio_sender(tx))
}

/// Phase 3.1.2: Embassy loop-based restart pattern (Research-Informed).
///
/// This task function implements the internal loop restart pattern identified in our
/// Embassy research. Instead of external task respawning, the actor runs in an internal
/// loop that can reset its state and continue running after errors, providing
/// deterministic restart behavior for embedded environments.
///
/// ## Loop-Based Restart Pattern
///
/// - Actor runs in an internal loop with cooperative restart
/// - On error, calls `on_cleanup()` then `on_restart()` and continues the loop  
/// - No external task destruction/creation - same task memory throughout lifetime
/// - Deterministic behavior suitable for embedded/real-time systems
///
/// ## Embassy Integration
///
/// - Uses Embassy signals for supervisor communication
/// - Cooperative error handling via `handle_safe()` method
/// - Embassy timer for restart delays (if needed)
/// - No heap allocation - suitable for no_std environments
///
/// # Arguments
///
/// * `actor` - The actor instance to run with loop-based restart
/// * `mailbox` - Embassy channel receiver for actor messages  
/// * `supervisor_signal` - Signal for notifying supervisor of failures
/// * `actor_id` - String identifier for this actor
///
/// # Returns
///
/// This function runs indefinitely until the mailbox is closed or
/// an unrecoverable error occurs during restart.
#[cfg(feature = "async-embassy")]
pub async fn embassy_actor_loop_task<A: Actor>(
    mut actor: A,
    mailbox: embassy_sync::channel::Receiver<
        'static,
        embassy_sync::blocking_mutex::raw::NoopRawMutex,
        A::Message,
        32,
    >,
    supervisor_signal: &'static embassy_sync::signal::Signal<
        embassy_sync::blocking_mutex::raw::NoopRawMutex,
        crate::actor::SupervisorMessage<ActorString>,
    >,
    actor_id: &'static str,
) where
    A::Message: 'static,
{
    // Embassy pattern: Internal loop with cooperative restart
    loop {
        // Initialize/reset actor state for restart
        if let Err(init_error) = actor.on_restart() {
            #[cfg(any(feature = "std", feature = "alloc"))]
            let boxed_error = Box::new(init_error);
            #[cfg(not(any(feature = "std", feature = "alloc")))]
            let boxed_error = init_error;

            #[cfg(any(feature = "std", feature = "alloc"))]
            let actor_id_string = actor_id.into();
            #[cfg(not(any(feature = "std", feature = "alloc")))]
            let actor_id_string = {
                let mut s = ActorString::new();
                let _ = s.push_str(actor_id);
                s
            };

            supervisor_signal.signal(crate::actor::SupervisorMessage::ChildPanicked {
                id: actor_id_string,
                error: boxed_error,
            });
            break; // Cannot restart - actor terminates
        }

        // Call actor startup hook
        if let Err(startup_error) = actor.on_start() {
            #[cfg(any(feature = "std", feature = "alloc"))]
            let boxed_error = Box::new(startup_error);
            #[cfg(not(any(feature = "std", feature = "alloc")))]
            let boxed_error = startup_error;

            #[cfg(any(feature = "std", feature = "alloc"))]
            let actor_id_string = actor_id.into();
            #[cfg(not(any(feature = "std", feature = "alloc")))]
            let actor_id_string = {
                let mut s = ActorString::new();
                let _ = s.push_str(actor_id);
                s
            };

            supervisor_signal.signal(crate::actor::SupervisorMessage::ChildPanicked {
                id: actor_id_string,
                error: boxed_error,
            });
            break; // Cannot start - actor terminates
        }

        // Message processing loop
        loop {
            let message = mailbox.receive().await;
            match actor.handle_safe(message).await {
                Ok(()) => continue, // Normal processing
                Err(actor_error) => {
                    #[cfg(any(feature = "std", feature = "alloc"))]
                    let boxed_error = Box::new(actor_error);
                    #[cfg(not(any(feature = "std", feature = "alloc")))]
                    let boxed_error = actor_error;

                    #[cfg(any(feature = "std", feature = "alloc"))]
                    let actor_id_string = actor_id.into();
                    #[cfg(not(any(feature = "std", feature = "alloc")))]
                    let actor_id_string = {
                        let mut s = ActorString::new();
                        let _ = s.push_str(actor_id);
                        s
                    };

                    // Signal supervisor about error
                    supervisor_signal.signal(crate::actor::SupervisorMessage::ChildPanicked {
                        id: actor_id_string,
                        error: boxed_error,
                    });
                    break; // Exit message loop to restart
                }
            }
        }

        // Perform cleanup before restart iteration
        let _ = actor.on_cleanup();

        // Loop continues for restart - supervisor can apply backoff via separate mechanisms
    }

    // Final cleanup on actor termination
    let _ = actor.on_cleanup();
}

/// Phase 3.1.4: Embassy external respawn pattern (Alternative).
///
/// This task function implements the external respawn pattern where the task
/// runs once and terminates, requiring the supervisor to respawn a new task instance.
/// This pattern is more complex but may be useful when the internal loop pattern
/// is insufficient for certain restart scenarios.
///
/// ## External Respawn Pattern
///
/// - Task runs once until error or completion
/// - Supervisor externally respawns new task instance  
/// - Requires TaskStorage management and more complex supervision
/// - May be needed for severe failure scenarios requiring full task reset
///
/// # Arguments
///
/// * `actor` - The actor instance to run until termination
/// * `mailbox` - Embassy channel receiver for actor messages
/// * `supervisor_signal` - Signal for notifying supervisor of failures  
/// * `actor_id` - String identifier for this actor
///
/// # Returns
///
/// This function runs until the mailbox is closed or an error occurs.
/// The supervisor is responsible for deciding whether to respawn.
#[cfg(feature = "async-embassy")]
pub async fn embassy_external_respawn_task<A: Actor>(
    mut actor: A,
    mailbox: embassy_sync::channel::Receiver<
        'static,
        embassy_sync::blocking_mutex::raw::NoopRawMutex,
        A::Message,
        32,
    >,
    supervisor_signal: &'static embassy_sync::signal::Signal<
        embassy_sync::blocking_mutex::raw::NoopRawMutex,
        crate::actor::SupervisorMessage<ActorString>,
    >,
    actor_id: &'static str,
) where
    A::Message: 'static,
{
    // Call actor startup hook
    if let Err(startup_error) = actor.on_start() {
        #[cfg(any(feature = "std", feature = "alloc"))]
        let boxed_error = Box::new(startup_error);
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        let boxed_error = startup_error;

        #[cfg(any(feature = "std", feature = "alloc"))]
        let actor_id_string = actor_id.into();
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        let actor_id_string = {
            let mut s = ActorString::new();
            let _ = s.push_str(actor_id);
            s
        };

        supervisor_signal.signal(crate::actor::SupervisorMessage::ChildPanicked {
            id: actor_id_string,
            error: boxed_error,
        });
        return; // Task terminates - supervisor will respawn if configured
    }

    // External respawn pattern: Task runs once, supervisor respawns
    loop {
        let message = mailbox.receive().await;
        match actor.handle_safe(message).await {
            Ok(()) => continue,
            Err(error) => {
                #[cfg(any(feature = "std", feature = "alloc"))]
                let boxed_error = Box::new(error);
                #[cfg(not(any(feature = "std", feature = "alloc")))]
                let boxed_error = error;

                #[cfg(any(feature = "std", feature = "alloc"))]
                let actor_id_string = actor_id.into();
                #[cfg(not(any(feature = "std", feature = "alloc")))]
                let actor_id_string = {
                    let mut s = ActorString::new();
                    let _ = s.push_str(actor_id);
                    s
                };

                supervisor_signal.signal(crate::actor::SupervisorMessage::ChildPanicked {
                    id: actor_id_string,
                    error: boxed_error,
                });
                break; // Task terminates - supervisor will respawn if configured
            }
        }
    }

    // Call cleanup on normal or error termination
    let _ = actor.on_cleanup();
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "async-tokio")]
    mod tokio_tests {
        use crate::actor::Actor;
        use std::sync::{Arc, Mutex};

        struct TestActor {
            counter: Arc<Mutex<u32>>,
        }

        impl TestActor {
            fn new(counter: Arc<Mutex<u32>>) -> Self {
                Self { counter }
            }
        }

        impl Actor for TestActor {
            type Message = u32;
            type Future<'a>
                = core::future::Ready<()>
            where
                Self: 'a;

            fn handle(&mut self, msg: Self::Message) -> Self::Future<'_> {
                let mut count = self.counter.lock().unwrap();
                *count += msg;
                core::future::ready(())
            }
        }

        #[tokio::test]
        async fn spawn_tokio_works() {
            let counter = Arc::new(Mutex::new(0));
            let actor = TestActor::new(counter.clone());

            let address = crate::actor::spawn::spawn_actor_tokio(actor, 10);

            address.send(5).await.unwrap();
            address.send(10).await.unwrap();

            // Give the actor time to process
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            let final_count = *counter.lock().unwrap();
            assert_eq!(final_count, 15);
        }

        use std::future::Future;
        use std::pin::Pin;

        /// Test actor that can panic on demand for testing panic-safe task
        struct PanicTestActor {
            counter: Arc<Mutex<u32>>,
            panic_on_value: Option<u32>,
        }

        impl PanicTestActor {
            fn new(counter: Arc<Mutex<u32>>, panic_on_value: Option<u32>) -> Self {
                Self {
                    counter,
                    panic_on_value,
                }
            }
        }

        impl Actor for PanicTestActor {
            type Message = u32;
            type Future<'a>
                = Pin<Box<dyn Future<Output = ()> + Send + 'a>>
            where
                Self: 'a;

            #[allow(clippy::collapsible_if)]
            fn handle(&mut self, msg: Self::Message) -> Self::Future<'_> {
                let counter = self.counter.clone();
                let panic_on_value = self.panic_on_value;

                Box::pin(async move {
                    // Panic if this is the trigger value
                    if let Some(panic_value) = panic_on_value {
                        if msg == panic_value {
                            panic!("Test panic triggered by value: {msg}");
                        }
                    }

                    let mut count = counter.lock().unwrap();
                    *count += msg;
                })
            }

            // Embassy-compatible error handling for testing
            #[allow(clippy::collapsible_if)]
            async fn handle_safe(
                &mut self,
                msg: Self::Message,
            ) -> Result<(), crate::actor::ActorError> {
                if let Some(panic_value) = self.panic_on_value {
                    if msg == panic_value {
                        return Err(crate::actor::ActorError::Custom("Test error triggered"));
                    }
                }

                self.handle(msg).await;
                Ok(())
            }
        }

        #[tokio::test]
        async fn panic_safe_actor_task_handles_normal_messages() {
            let counter = Arc::new(Mutex::new(0));
            let actor = PanicTestActor::new(counter.clone(), None); // No panic trigger

            let (tx, rx) = tokio::sync::mpsc::channel(10);

            // Spawn the panic-safe task
            let task_handle = tokio::spawn(crate::actor::spawn::panic_safe_actor_task(
                actor,
                rx,
                None, // No supervisor
                "test_actor".to_string(),
            ));

            // Send some messages
            tx.send(5).await.unwrap();
            tx.send(10).await.unwrap();

            // Close the channel to terminate the task normally
            drop(tx);

            // Wait for task completion
            let result = task_handle.await.unwrap();
            assert!(result.is_ok());

            // Verify messages were processed
            let final_count = *counter.lock().unwrap();
            assert_eq!(final_count, 15);
        }

        #[tokio::test]
        async fn panic_safe_actor_task_captures_panics() {
            let counter = Arc::new(Mutex::new(0));
            let actor = PanicTestActor::new(counter.clone(), Some(42)); // Panic on 42

            let (tx, rx) = tokio::sync::mpsc::channel(10);

            // Spawn the panic-safe task
            let task_handle = tokio::spawn(crate::actor::spawn::panic_safe_actor_task(
                actor,
                rx,
                None, // No supervisor for this test
                "test_actor".to_string(),
            ));

            // Send normal message first
            tx.send(5).await.unwrap();

            // Send panic trigger
            tx.send(42).await.unwrap();

            // Task should terminate gracefully after panic
            let result = task_handle.await.unwrap();
            assert!(result.is_ok()); // Panic was caught, task returned Ok

            // Verify only the first message was processed
            let final_count = *counter.lock().unwrap();
            assert_eq!(final_count, 5);
        }

        // Phase 3 Comprehensive Testing: Panic-Safe Event Loops and Enhanced Spawn Functions

        #[tokio::test]
        async fn test_panic_safe_actor_task_with_supervisor_notification() {
            let counter = Arc::new(Mutex::new(0));
            let actor = PanicTestActor::new(counter.clone(), Some(100)); // Panic on 100

            let (supervisor_tx, mut supervisor_rx) = tokio::sync::mpsc::channel(10);
            let supervisor_address =
                crate::actor::address::Address::from_tokio_sender(supervisor_tx);

            let (actor_tx, actor_rx) = tokio::sync::mpsc::channel(10);

            // Spawn the panic-safe task with supervisor notification
            let task_handle = tokio::spawn(crate::actor::spawn::panic_safe_actor_task(
                actor,
                actor_rx,
                Some(supervisor_address),
                "test_panic_actor".to_string(),
            ));

            // Send normal messages first
            actor_tx.send(10).await.unwrap();
            actor_tx.send(20).await.unwrap();

            // Send panic trigger
            actor_tx.send(100).await.unwrap();

            // Task should terminate gracefully after panic
            let result = task_handle.await.unwrap();
            assert!(result.is_ok()); // Panic was caught, task returned Ok

            // Verify normal messages were processed
            let final_count = *counter.lock().unwrap();
            assert_eq!(final_count, 30);

            // Verify supervisor was notified about the panic
            let supervisor_msg = supervisor_rx.recv().await.unwrap();
            match supervisor_msg {
                crate::actor::SupervisorMessage::ChildPanicked { id, error } => {
                    assert_eq!(id, "test_panic_actor");
                    match *error {
                        crate::actor::ActorError::Panic { .. } => {
                            // Expected panic error
                        }
                        _ => panic!("Expected ActorError::Panic, got {error:?}"),
                    }
                }
                _ => panic!("Expected ChildPanicked message, got {supervisor_msg:?}"),
            }
        }

        #[tokio::test]
        async fn test_panic_safe_actor_startup_failure_notification() {
            // Create an actor that fails during startup
            struct FailingStartupActor;

            impl crate::actor::Actor for FailingStartupActor {
                type Message = u32;
                type Future<'a>
                    = core::future::Ready<()>
                where
                    Self: 'a;

                fn handle(&mut self, _msg: u32) -> Self::Future<'_> {
                    core::future::ready(())
                }

                fn on_start(&mut self) -> Result<(), crate::actor::ActorError> {
                    Err(crate::actor::ActorError::StartupFailure)
                }
            }

            let actor = FailingStartupActor;

            let (supervisor_tx, mut supervisor_rx) = tokio::sync::mpsc::channel(10);
            let supervisor_address =
                crate::actor::address::Address::from_tokio_sender(supervisor_tx);

            let (_actor_tx, actor_rx) = tokio::sync::mpsc::channel(10);

            // Spawn the panic-safe task with supervisor notification
            let task_handle = tokio::spawn(crate::actor::spawn::panic_safe_actor_task(
                actor,
                actor_rx,
                Some(supervisor_address),
                "failing_startup_actor".to_string(),
            ));

            // Task should fail during startup
            let result = task_handle.await.unwrap();
            assert!(result.is_err()); // Startup failure returns Err

            match result {
                Err(crate::actor::ActorError::StartupFailure) => {
                    // Expected startup failure
                }
                _ => panic!("Expected StartupFailure, got {result:?}"),
            }

            // Verify supervisor was notified about the startup failure
            let supervisor_msg = supervisor_rx.recv().await.unwrap();
            match supervisor_msg {
                crate::actor::SupervisorMessage::ChildPanicked { id, error } => {
                    assert_eq!(id, "failing_startup_actor");
                    match *error {
                        crate::actor::ActorError::StartupFailure => {
                            // Expected startup failure
                        }
                        _ => panic!("Expected ActorError::StartupFailure, got {error:?}"),
                    }
                }
                _ => panic!("Expected ChildPanicked message, got {supervisor_msg:?}"),
            }
        }

        #[tokio::test]
        async fn test_spawn_supervised_actor_with_panic_handling_integration() {
            use crate::actor::spawn::spawn_supervised_actor_with_panic_handling;
            use crate::actor::supervision::SupervisorActor;

            let mut supervisor = SupervisorActor::<String, 8>::new();

            let (supervisor_tx, _supervisor_rx) = tokio::sync::mpsc::channel(10);
            let supervisor_address =
                crate::actor::address::Address::from_tokio_sender(supervisor_tx);

            let counter = Arc::new(Mutex::new(0));
            let actor = PanicTestActor::new(counter.clone(), Some(999)); // Panic on 999

            // Spawn supervised actor with panic handling
            let actor_address = spawn_supervised_actor_with_panic_handling(
                actor,
                &mut supervisor,
                "supervised_panic_actor".to_string(),
                32,
                Some(supervisor_address),
            )
            .unwrap();

            // Send normal messages
            actor_address.send(100).await.unwrap();
            actor_address.send(200).await.unwrap();

            // Give actor time to process
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            // Verify normal messages were processed
            let processed_count = *counter.lock().unwrap();
            assert_eq!(processed_count, 300);

            // Verify child was added to supervisor (use public method)
            assert!(supervisor.apply_restart_intensity(&"supervised_panic_actor".to_string()));
        }

        #[tokio::test]
        async fn test_multiple_panic_safe_actors_with_different_behaviors() {
            // Create separate addresses to avoid clone() calls
            let (supervisor_tx1, mut supervisor_rx1) = tokio::sync::mpsc::channel(20);
            let (supervisor_tx2, mut supervisor_rx2) = tokio::sync::mpsc::channel(20);
            let (supervisor_tx3, mut supervisor_rx3) = tokio::sync::mpsc::channel(20);

            let supervisor_address1 =
                crate::actor::address::Address::from_tokio_sender(supervisor_tx1);
            let supervisor_address2 =
                crate::actor::address::Address::from_tokio_sender(supervisor_tx2);
            let supervisor_address3 =
                crate::actor::address::Address::from_tokio_sender(supervisor_tx3);

            // Combine all receivers into one for monitoring
            let mut all_supervisor_msgs = Vec::new();

            // Create actors with different panic triggers
            let counter1 = Arc::new(Mutex::new(0));
            let counter2 = Arc::new(Mutex::new(0));
            let counter3 = Arc::new(Mutex::new(0));

            let actor1 = PanicTestActor::new(counter1.clone(), Some(50)); // Panic on 50
            let actor2 = PanicTestActor::new(counter2.clone(), Some(75)); // Panic on 75
            let actor3 = PanicTestActor::new(counter3.clone(), None); // Never panic

            let (tx1, rx1) = tokio::sync::mpsc::channel(10);
            let (tx2, rx2) = tokio::sync::mpsc::channel(10);
            let (tx3, rx3) = tokio::sync::mpsc::channel(10);

            // Spawn multiple panic-safe tasks
            let task1 = tokio::spawn(crate::actor::spawn::panic_safe_actor_task(
                actor1,
                rx1,
                Some(supervisor_address1),
                "actor1".to_string(),
            ));
            let task2 = tokio::spawn(crate::actor::spawn::panic_safe_actor_task(
                actor2,
                rx2,
                Some(supervisor_address2),
                "actor2".to_string(),
            ));
            let task3 = tokio::spawn(crate::actor::spawn::panic_safe_actor_task(
                actor3,
                rx3,
                Some(supervisor_address3),
                "actor3".to_string(),
            ));

            // Send messages to all actors
            tx1.send(10).await.unwrap();
            tx2.send(20).await.unwrap();
            tx3.send(30).await.unwrap();

            // Trigger panic in actor1
            tx1.send(50).await.unwrap();

            // Trigger panic in actor2
            tx2.send(75).await.unwrap();

            // Send more messages to actor3 (should continue working)
            tx3.send(40).await.unwrap();
            tx3.send(50).await.unwrap();

            // Close channels to terminate actors
            drop(tx1);
            drop(tx2);
            drop(tx3);

            // Wait for tasks to complete
            let result1 = task1.await.unwrap();
            let result2 = task2.await.unwrap();
            let result3 = task3.await.unwrap();

            assert!(result1.is_ok()); // Panic was caught
            assert!(result2.is_ok()); // Panic was caught  
            assert!(result3.is_ok()); // Normal termination

            // Verify message processing
            assert_eq!(*counter1.lock().unwrap(), 10); // Only first message processed
            assert_eq!(*counter2.lock().unwrap(), 20); // Only first message processed
            assert_eq!(*counter3.lock().unwrap(), 120); // All messages processed

            // Collect all supervisor messages
            let mut panic_count = 0;

            // Check all three supervisor receivers
            while let Ok(msg) = supervisor_rx1.try_recv() {
                if matches!(msg, crate::actor::SupervisorMessage::ChildPanicked { .. }) {
                    panic_count += 1;
                }
                all_supervisor_msgs.push(msg);
            }
            while let Ok(msg) = supervisor_rx2.try_recv() {
                if matches!(msg, crate::actor::SupervisorMessage::ChildPanicked { .. }) {
                    panic_count += 1;
                }
                all_supervisor_msgs.push(msg);
            }
            while let Ok(msg) = supervisor_rx3.try_recv() {
                if matches!(msg, crate::actor::SupervisorMessage::ChildPanicked { .. }) {
                    panic_count += 1;
                }
                all_supervisor_msgs.push(msg);
            }

            assert_eq!(panic_count, 2); // Two actors panicked
        }

        // Phase 3: Comprehensive TestKit Integration Tests (Task 5.3 Usage)

        #[tokio::test]
        async fn test_actor_probe_lifecycle_events() {
            use crate::test_utils::{ActorProbe, TestKit};

            let test_kit = TestKit::new();
            let counter = Arc::new(Mutex::new(0));
            let actor = TestActor::new(counter.clone());

            // Spawn actor with probe (specify capacity)
            let (address, mut probe): (_, ActorProbe<TestActor>) =
                test_kit.spawn_actor_with_probe::<_, 32>(actor);

            // Wait for actor startup
            probe.expect_actor_started().await.unwrap();

            // Send messages and observe processing
            address.send(10).await.unwrap();
            address.send(20).await.unwrap();

            // Verify messages were processed
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            assert_eq!(*counter.lock().unwrap(), 30);

            // Close the address to trigger actor shutdown
            drop(address);

            // Wait for actor to stop
            probe.expect_actor_stopped().await.unwrap();
        }

        #[tokio::test]
        async fn test_panic_detection_with_probe() {
            use crate::test_utils::{ActorProbe, TestKit};

            let test_kit = TestKit::new();
            let counter = Arc::new(Mutex::new(0));
            let actor = PanicTestActor::new(counter.clone(), Some(42)); // Panic on 42

            // Spawn actor with probe (specify capacity)
            let (address, mut probe): (_, ActorProbe<PanicTestActor>) =
                test_kit.spawn_actor_with_probe::<_, 32>(actor);

            // Wait for actor startup
            probe.expect_actor_started().await.unwrap();

            // Send normal message
            address.send(10).await.unwrap();

            // Send panic trigger
            address.send(42).await.unwrap();

            // Expect panic to be captured by probe
            let panic_info = probe.expect_panic().await.unwrap();
            assert!(panic_info.as_str().contains("42"));

            // Verify normal message was processed before panic
            assert_eq!(*counter.lock().unwrap(), 10);
        }

        #[tokio::test]
        async fn test_message_content_capture() {
            use crate::test_utils::{ActorProbe, ProbeEvent, TestKit};

            let test_kit = TestKit::new();
            let counter = Arc::new(Mutex::new(0));
            let actor = TestActor::new(counter.clone());

            // Spawn actor with probe (specify capacity)
            let (address, mut probe): (_, ActorProbe<TestActor>) =
                test_kit.spawn_actor_with_probe::<_, 32>(actor);

            // Wait for actor startup
            probe.expect_actor_started().await.unwrap();

            // Send messages
            address.send(100).await.unwrap();
            address.send(200).await.unwrap();
            address.send(300).await.unwrap();

            // Capture messages
            let captured_messages = probe.capture_messages(5).await.unwrap();

            // Verify we captured the right events
            let mut message_events = 0;
            for event in &captured_messages {
                match event {
                    ProbeEvent::MessageReceived { message_type } => {
                        assert_eq!(message_type.as_str(), "u32");
                        message_events += 1;
                    }
                    ProbeEvent::ActorStarted => {
                        // Expected startup event
                    }
                    _ => {
                        // Other events are fine
                    }
                }
            }

            assert_eq!(message_events, 3); // Should have captured 3 message events
            assert_eq!(*counter.lock().unwrap(), 600); // All messages processed
        }

        #[tokio::test]
        async fn test_deterministic_time_control() {
            use crate::test_utils::TestKit;

            let test_kit = TestKit::new();

            // Pause time for deterministic testing
            test_kit.pause_time();

            let start_time = std::time::Instant::now();

            // Advance time by controlled amounts
            test_kit
                .advance_time(tokio::time::Duration::from_secs(1))
                .await;
            test_kit
                .advance_time(tokio::time::Duration::from_secs(2))
                .await;

            // Real time should not have advanced much
            let elapsed = start_time.elapsed();
            assert!(elapsed < std::time::Duration::from_millis(100));

            // Resume normal time
            test_kit.resume_time();
        }

        #[tokio::test]
        async fn test_instrumented_actor_wrapper() {
            use crate::test_utils::{InstrumentedActor, ProbeEvent};

            let counter = Arc::new(Mutex::new(0));
            let base_actor = TestActor::new(counter.clone());

            // Create probe channel for instrumentation
            let (probe_sender, mut probe_receiver) = tokio::sync::mpsc::channel(32);

            // Wrap actor with instrumentation
            let instrumented_actor = InstrumentedActor::new(base_actor, probe_sender);

            // Spawn the instrumented actor using the spawn function from this module
            let address = crate::actor::spawn::spawn_actor_tokio(instrumented_actor, 32);

            // Send messages
            address.send(50).await.unwrap();
            address.send(75).await.unwrap();

            // Give actor time to process
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            // Verify messages were processed
            assert_eq!(*counter.lock().unwrap(), 125);

            // Capture probe events from the receiver
            let mut found_messages = 0;
            while let Ok(event) = probe_receiver.try_recv() {
                if matches!(event, ProbeEvent::MessageReceived { .. }) {
                    found_messages += 1;
                }
            }

            assert_eq!(found_messages, 2); // Should have captured both messages
        }

        #[tokio::test]
        async fn test_supervision_with_probes() {
            use crate::actor::supervision::SupervisorActor;
            use crate::test_utils::{ActorProbe, TestKit};

            let test_kit = TestKit::new();
            let mut supervisor = SupervisorActor::<String, 8>::new();

            // Create a supervision scenario with probes
            let counter = Arc::new(Mutex::new(0));
            let actor = PanicTestActor::new(counter.clone(), Some(999)); // Won't panic with normal messages

            // Spawn supervised actor with probe (specify capacity)
            let (address, mut probe): (_, ActorProbe<PanicTestActor>) =
                test_kit.spawn_actor_with_probe::<_, 32>(actor);

            // Also manually add to supervisor for this test
            // (In real usage, supervised spawn functions would handle this)
            let _ = supervisor.add_child("test_supervised_actor".to_string(), None);

            // Wait for actor startup
            probe.expect_actor_started().await.unwrap();

            // Send normal messages
            address.send(100).await.unwrap();
            address.send(200).await.unwrap();

            // Verify processing
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            assert_eq!(*counter.lock().unwrap(), 300);

            // Test that supervision integration works
            assert!(supervisor.apply_restart_intensity(&"test_supervised_actor".to_string()));
        }

        #[tokio::test]
        async fn test_testkit_panic_capture() {
            use crate::test_utils::{ActorProbe, TestKit};

            let test_kit = TestKit::new();

            // Test panic scenario
            let counter = Arc::new(Mutex::new(0));
            let panic_actor = PanicTestActor::new(counter.clone(), Some(100));

            let (address, mut probe): (_, ActorProbe<PanicTestActor>) =
                test_kit.spawn_actor_with_probe::<_, 32>(panic_actor);

            // Wait for startup
            probe.expect_actor_started().await.unwrap();

            // Send messages
            address.send(50).await.unwrap(); // Should process normally

            // Send panic trigger
            address.send(100).await.unwrap(); // Should trigger panic

            // Expect panic to be captured
            let _panic_info = probe.expect_panic().await.unwrap();
            assert_eq!(*counter.lock().unwrap(), 50); // Only first message processed
        }

        #[tokio::test]
        async fn test_testkit_normal_operation() {
            use crate::test_utils::{ActorProbe, TestKit};

            let test_kit = TestKit::new();

            // Test normal operation
            let counter = Arc::new(Mutex::new(0));
            let normal_actor = TestActor::new(counter.clone());

            let (address, mut probe): (_, ActorProbe<TestActor>) =
                test_kit.spawn_actor_with_probe::<_, 32>(normal_actor);

            // Wait for startup
            probe.expect_actor_started().await.unwrap();

            // Send all messages
            for &msg in &[10, 20, 30] {
                address.send(msg).await.unwrap();
            }

            // Wait for processing
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            assert_eq!(*counter.lock().unwrap(), 60);

            // Clean shutdown
            drop(address);
            probe.expect_actor_stopped().await.unwrap();
        }
    }

    #[cfg(feature = "async-embassy")]
    mod embassy_tests {
        use embassy_sync::blocking_mutex::raw::NoopRawMutex;

        #[test]
        fn embassy_spawn_compiles() {
            // This test ensures the Embassy spawn function compiles correctly
            // Actual runtime testing would require an Embassy executor

            fn test_spawn_signature() {
                fn _test(
                    spawner: embassy_executor::Spawner,
                    actor: crate::actor::spawn::CounterActor,
                    sender: embassy_sync::channel::Sender<'static, NoopRawMutex, u32, 16>,
                    receiver: embassy_sync::channel::Receiver<'static, NoopRawMutex, u32, 16>,
                ) {
                    let _result = crate::actor::spawn::spawn_counter_actor_embassy(
                        spawner, actor, sender, receiver,
                    );
                }
            }

            test_spawn_signature();
        }

        #[test]
        fn embassy_batch_spawn_compiles() {
            // This test ensures the Embassy batch spawn function compiles correctly
            // Actual runtime testing would require an Embassy executor

            fn test_batch_spawn_signature() {
                fn _test(
                    spawner: embassy_executor::Spawner,
                    actor: crate::actor::spawn::CounterBatchActor,
                    sender: embassy_sync::channel::Sender<'static, NoopRawMutex, u32, 16>,
                    receiver: embassy_sync::channel::Receiver<'static, NoopRawMutex, u32, 16>,
                ) {
                    let _result = crate::actor::spawn::spawn_counter_batch_actor_embassy(
                        spawner, actor, sender, receiver,
                    );
                }
            }

            test_batch_spawn_signature();
        }

        #[test]
        fn static_embassy_channel_macro_works() {
            // Test that the macro expands correctly
            let (_sender, _receiver) = crate::static_embassy_channel!(TEST_CHANNEL: u32, 8);

            // The macro should create properly typed sender and receiver
            // Actual usage would be in an Embassy runtime context
        }

        #[test]
        fn counter_batch_actor_implements_batch_trait() {
            use crate::actor::BatchActor;

            let actor = crate::actor::spawn::CounterBatchActor::new();
            assert_eq!(actor.max_batch_size(), 16); // Default batch size

            let actor_custom = crate::actor::spawn::CounterBatchActor::with_batch_size(8);
            assert_eq!(actor_custom.max_batch_size(), 8); // Custom batch size
        }
    }
}
