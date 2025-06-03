use criterion::{Criterion, Throughput, criterion_group};
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
}

impl AsyncTestActor {
    fn new() -> Self {
        Self { count: 0 }
    }
}

impl Actor for AsyncTestActor {
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

pub fn bench_message_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_throughput");
    group.throughput(Throughput::Elements(1000));

    // Sync baseline
    group.bench_function("sync_1000_msgs", |b| {
        b.iter(|| {
            let actor = SyncTestActor::new();
            let addr = lit_bit_core::actor::spawn_actor_tokio(actor, 1000);
            for i in 0..1000 {
                addr.try_send(TestMessage(i)).unwrap();
            }
        });
    });

    // Async comparison with Tokio
    let rt = TokioBuilder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    group.bench_function("async_1000_msgs", |b| {
        b.iter(|| {
            rt.block_on(async {
                let actor = AsyncTestActor::new();
                let addr = lit_bit_core::actor::spawn_actor_tokio(actor, 1000);
                for i in 0..1000 {
                    addr.send(TestMessage(i)).await.unwrap();
                }
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
