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
        .unwrap();

    // Measure actor creation time
    group.bench_function("actor_creation", |b| {
        b.iter(|| {
            let actor = TestActor::new();
            let _addr = lit_bit_core::actor::spawn_actor_tokio(actor, 16);
        });
    });

    // Measure message sending time
    group.bench_function("message_send", |b| {
        let actor = TestActor::new();
        let addr = lit_bit_core::actor::spawn_actor_tokio(actor, 16);

        b.iter(|| {
            rt.block_on(async {
                addr.send(TestMessage(0)).await.unwrap();
            });
        });
    });

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default();
    targets = bench_actor_performance
);
