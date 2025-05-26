//! Actor spawning functions for Embassy and Tokio runtimes.

#[cfg(all(not(feature = "std"), feature = "embassy"))]
use super::{Actor, actor_task, create_mailbox};

#[cfg(feature = "std")]
use super::{Actor, actor_task, create_mailbox};

#[cfg(any(all(not(feature = "std"), feature = "embassy"), feature = "std"))]
use super::address::Address;

// Embassy spawning function (Task 3.2)
#[cfg(all(not(feature = "std"), feature = "embassy"))]
pub fn spawn_actor_embassy<A, const N: usize>(
    spawner: embassy_executor::Spawner,
    actor: A,
) -> Address<A::Message, N>
where
    A: Actor + 'static,
    A::Message: 'static,
{
    let (outbox, inbox) = create_mailbox::<A::Message, N>();

    // Move actor and inbox into static context (Embassy requirement)
    spawner.spawn(actor_task::<A, N>(actor, inbox)).unwrap();

    Address::from_producer(outbox)
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
    use super::*;

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

    #[cfg(feature = "std")]
    #[tokio::test]
    async fn spawn_tokio_works() {
        let actor = TestActor::new();
        let addr = spawn_actor_tokio::<_, 16>(actor);

        // Test that we can send a message
        addr.send(42).await.unwrap();

        // Give the actor time to process
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
}
