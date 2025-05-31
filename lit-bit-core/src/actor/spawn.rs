//! Actor spawning functions for Embassy and Tokio runtimes.

#[cfg(feature = "async-embassy")]
use super::{Actor, BatchActor};

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

#[cfg(test)]
mod tests {
    #[cfg(all(feature = "async-tokio", not(feature = "async-embassy")))]
    mod tokio_tests {
        use super::super::{Actor, spawn_actor_tokio};
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
            let shared_counter = Arc::new(Mutex::new(0u32));
            let actor = TestActor::new(Arc::clone(&shared_counter));
            let actor_address = spawn_actor_tokio(actor, 16);

            // Test that we can send a message
            actor_address.send(42).await.unwrap();

            // Give the actor time to process the message
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            // Verify that the message was processed by checking the counter
            let final_count = *shared_counter.lock().unwrap();
            assert_eq!(
                final_count, 42,
                "Actor should have processed the message and updated counter to 42"
            );
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
