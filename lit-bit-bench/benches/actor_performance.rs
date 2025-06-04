use criterion::{Criterion, Throughput, criterion_group};
use lit_bit_core::actor::Actor;
use tokio::runtime::Builder as TokioBuilder;

// Test message type
#[derive(Debug, Clone)]
struct TestMessage(u32);

// Test actor
struct TestActor {
    count: u32,
}

impl TestActor {
    fn new() -> Self {
        Self { count: 0 }
    }
}

impl Actor for TestActor {
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

pub fn bench_actor_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("actor_performance");
    group.throughput(Throughput::Elements(1));

    // Create a Tokio runtime for async operations
    let rt = TokioBuilder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime for benchmarks");

    // Measure actor creation and cleanup time
    group.bench_function("actor_creation_and_cleanup", |b| {
        b.iter(|| {
            // Create and spawn actor
            let actor = TestActor::new();
            let addr = lit_bit_core::actor::spawn_actor_tokio(actor, 16);

            // Ensure cleanup by explicitly dropping the address
            drop(addr);
        });
    });

    // Measure message sending time with fresh actor per iteration
    group.bench_function("message_send", |b| {
        b.iter_with_setup(
            // Setup: Create fresh actor for each iteration
            || {
                let actor = TestActor::new();
                lit_bit_core::actor::spawn_actor_tokio(actor, 16)
            },
            // Benchmark: Send message and ensure actor processes it
            |addr| {
                rt.block_on(async {
                    addr.send(TestMessage(1))
                        .await
                        .expect("Failed to send message to actor");
                });

                // Cleanup after each iteration
                drop(addr);
            },
        );
    });

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default();
    targets = bench_actor_performance
);
