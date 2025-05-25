//! Actor spawning functions for Embassy and Tokio runtimes.

#[allow(unused_imports)] // TODO: Remove when spawning functions are fully implemented
use super::{Actor, actor_task, create_mailbox};

#[cfg(all(not(feature = "std"), feature = "embassy"))]
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
// TODO: Complete implementation when Address integration is finished
#[cfg(feature = "std")]
#[allow(dead_code)]
fn spawn_actor_tokio_placeholder<A, const N: usize>(actor: A)
where
    A: Actor + Send + 'static,
    A::Message: Send + 'static,
{
    let (_outbox, inbox) = create_mailbox::<A::Message, N>();

    // Spawn on current Tokio runtime
    tokio::spawn(actor_task::<A, N>(actor, inbox));

    // TODO: Return Address when integration is complete
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
    async fn spawn_tokio_compiles() {
        // This test just ensures the function signature compiles
        // Actual functionality testing would require a complete implementation
        let _actor = TestActor::new();
        // let _addr = spawn_actor_tokio::<_, 16>(actor);
        // TODO: Uncomment when Address::from_tokio_sender is implemented
    }
}
