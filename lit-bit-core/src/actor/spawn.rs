//! Actor spawning functions for Embassy and Tokio runtimes.

#[cfg(feature = "async-embassy")]
use super::Actor;

#[cfg(all(feature = "async-tokio", not(feature = "async-embassy")))]
use super::{Actor, actor_task, create_mailbox};

#[cfg(any(feature = "async-embassy", feature = "async-tokio"))]
use super::address::Address;

// Embassy-specific imports
#[cfg(feature = "async-embassy")]
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
#[cfg(feature = "async-embassy")]
use embassy_sync::channel::Receiver;

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
#[cfg(all(feature = "async-tokio", not(feature = "async-embassy")))]
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
        fn static_embassy_channel_macro_works() {
            // Test that the macro expands correctly
            let (_sender, _receiver) = crate::static_embassy_channel!(TEST_CHANNEL: u32, 8);

            // The macro should create properly typed sender and receiver
            // Actual usage would be in an Embassy runtime context
        }
    }
}
