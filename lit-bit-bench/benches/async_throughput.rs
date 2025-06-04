use criterion::{Criterion, Throughput, criterion_group};
use futures::future::join_all;
use lit_bit_core::actor::Actor;
use tokio::runtime::Builder as TokioBuilder;

// Test message type
#[derive(Debug, Clone)]
struct TestMessage(u32);

// Sync test actor
struct SyncTestActor {
    count: u32,
}

impl SyncTestActor {
    fn new() -> Self {
        Self { count: 0 }
    }
}

impl Actor for SyncTestActor {
    type Message = TestMessage;
    type Future<'a>
        = std::future::Ready<()>
    where
        Self: 'a;

    fn handle(&mut self, msg: Self::Message) -> Self::Future<'_> {
        self.count += msg.0;
        std::future::ready(())
    }
}

// Async test actor
struct AsyncTestActor {
    count: u32,
    delay_ms: u64, // Added delay for async simulation
}

impl AsyncTestActor {
    fn new() -> Self {
        Self {
            count: 0,
            delay_ms: 1, // 1ms delay per message
        }
    }
}

impl Actor for AsyncTestActor {
    type Message = TestMessage;
    type Future<'a>
        = std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>>
    where
        Self: 'a;

    fn handle(&mut self, msg: Self::Message) -> Self::Future<'_> {
        let delay = self.delay_ms;
        let count = self.count;

        // Box the async block to return a proper future
        Box::pin(async move {
            // Simulate some async work with a delay
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;

            // Do some additional async processing
            let _result = tokio::task::yield_now().await;

            // Update state after async work
            self.count = count + msg.0;
        })
    }
}

pub fn bench_message_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_throughput");
    group.throughput(Throughput::Elements(1000));

    // Create a shared runtime for all async tests
    let rt = TokioBuilder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    // Non-blocking message sending benchmark
    group.bench_function("nonblocking_try_send_1000_msgs", |b| {
        b.iter(|| {
            rt.block_on(async {
                let actor = SyncTestActor::new();
                let addr = lit_bit_core::actor::spawn_actor_tokio(actor, 1000);

                // Send messages non-blocking
                for i in 0..1000 {
                    addr.try_send(TestMessage(i)).unwrap();
                }

                // Ensure cleanup by dropping the address
                drop(addr);
            })
        });
    });

    // Concurrent async message sending benchmark
    group.bench_function("concurrent_async_1000_msgs", |b| {
        b.iter(|| {
            rt.block_on(async {
                let actor = AsyncTestActor::new();
                let addr = lit_bit_core::actor::spawn_actor_tokio(actor, 1000);

                // Create futures for concurrent message sending
                let futures: Vec<_> = (0..1000)
                    .map(|i| {
                        let msg = TestMessage(i);
                        addr.send(msg)
                    })
                    .collect();

                // Send all messages concurrently
                join_all(futures).await;

                // Ensure cleanup
                drop(addr);
            })
        });
    });

    // Sequential async message sending benchmark (for comparison)
    group.bench_function("sequential_async_1000_msgs", |b| {
        b.iter(|| {
            rt.block_on(async {
                let actor = AsyncTestActor::new();
                let addr = lit_bit_core::actor::spawn_actor_tokio(actor, 1000);

                // Send messages sequentially
                for i in 0..1000 {
                    addr.send(TestMessage(i)).await.unwrap();
                }

                // Ensure cleanup
                drop(addr);
            })
        });
    });

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default();
    targets = bench_message_throughput
);
