//! Actor spawning functions for Embassy and Tokio runtimes.

#[cfg(all(not(feature = "std"), feature = "embassy"))]
use super::{Actor, actor_task};

#[cfg(feature = "std")]
use super::{Actor, actor_task, create_mailbox};

#[cfg(any(all(not(feature = "std"), feature = "embassy"), feature = "std"))]
use super::address::Address;

// Embassy task for running actors (must be at top level)
#[cfg(all(not(feature = "std"), feature = "embassy"))]
#[embassy_executor::task]
async fn embassy_actor_task<A: Actor + 'static, const N: usize>(
    actor: A,
    inbox: super::Inbox<A::Message, N>,
) {
    let _ = actor_task::<A, N>(actor, inbox).await;
}

// Embassy spawning function (Task 3.2)
#[cfg(all(not(feature = "std"), feature = "embassy"))]
/// Spawns an actor on the Embassy executor using a statically allocated mailbox.
///
/// This function requires the caller to provide a static mailbox using the `static_mailbox!` macro.
/// This ensures no heap allocation and gives users full control over memory placement.
///
/// # Arguments
///
/// * `spawner` - The Embassy spawner to use for spawning the actor task
/// * `actor` - The actor instance to spawn
/// * `outbox` - The producer end of a static mailbox (from `static_mailbox!`)
/// * `inbox` - The consumer end of a static mailbox (from `static_mailbox!`)
///
/// # Returns
///
/// Returns `Ok(Address)` if the actor was successfully spawned, or `Err(embassy_executor::SpawnError)`
/// if spawning failed (e.g., task arena is full).
///
/// # Examples
///
/// ```rust,no_run
/// use embassy_executor::Spawner;
/// use lit_bit_core::{actor::spawn_actor_embassy, static_mailbox};
///
/// fn spawn_my_actor(spawner: Spawner, actor: MyActor) -> Result<Address<MyMessage, 16>, embassy_executor::SpawnError> {
///     let (outbox, inbox) = static_mailbox!(ACTOR_QUEUE: MyMessage, 16);
///     spawn_actor_embassy(spawner, actor, outbox, inbox)
/// }
/// ```
pub fn spawn_actor_embassy<A, const N: usize>(
    spawner: embassy_executor::Spawner,
    actor: A,
    outbox: super::Outbox<A::Message, N>,
    inbox: super::Inbox<A::Message, N>,
) -> Result<Address<A::Message, N>, embassy_executor::SpawnError>
where
    A: Actor + 'static,
    A::Message: 'static,
{
    // Spawn the embassy task - return error instead of panicking
    spawner.spawn(embassy_actor_task(actor, inbox))?;

    Ok(Address::from_producer(outbox))
}

// Tokio spawning function (Task 3.3)
#[cfg(feature = "std")]
pub fn spawn_actor_tokio<A, const N: usize>(actor: A) -> Address<A::Message, N>
where
    A: Actor + Send + 'static,
    A::Message: Send + 'static,
{
    let (outbox, inbox) = create_mailbox::<A::Message, N>();

    // Spawn on current Tokio runtime
    tokio::spawn(actor_task::<A, N>(actor, inbox));

    // Create Address from the Tokio sender
    Address::from_tokio_sender(outbox)
}

// Graceful termination patterns (Task 3.4)
// This would typically be implemented as part of the event enum in user code:
//
// enum MyEvent {
//     DoWork(WorkData),
//     Terminate, // Special termination event
// }
//
// impl Actor for MyActor {
//     async fn on_event(&mut self, event: MyEvent) {
//         match event {
//             MyEvent::Terminate => return, // Break out of processing loop
//             _ => { /* handle normal events */ }
//         }
//     }
// }

#[cfg(test)]
mod tests {
    #[cfg(feature = "std")]
    mod std_tests {
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

            #[allow(clippy::manual_async_fn)] // Need Send bound for thread safety
            fn on_event(&mut self, msg: u32) -> impl core::future::Future<Output = ()> + Send {
                let counter = Arc::clone(&self.counter);
                async move {
                    let mut count = counter.lock().unwrap();
                    *count += msg;
                }
            }
        }

        #[tokio::test]
        async fn spawn_tokio_works() {
            let shared_counter = Arc::new(Mutex::new(0u32));
            let actor = TestActor::new(Arc::clone(&shared_counter));
            let actor_address = spawn_actor_tokio::<_, 16>(actor);

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
}
