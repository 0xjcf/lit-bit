//! Memory usage benchmarks for statechart and actor systems

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use lit_bit_bench::utils::TrackingAllocator;
use lit_bit_core::actor::{Actor, create_mailbox};

// Note: For more advanced zero-allocation testing, a custom global allocator
// could be implemented here to track allocation counts globally

// Test actor for memory benchmarks
#[derive(Debug)]
struct BenchActor {
    state: u32,
    #[allow(dead_code)] // Used for memory measurement purposes
    data: Vec<u8>,
}

impl BenchActor {
    fn new(size: usize) -> Self {
        Self {
            state: 0,
            data: vec![0u8; size],
        }
    }
}

impl Actor for BenchActor {
    type Message = u32;

    fn on_event(&mut self, msg: u32) -> futures::future::BoxFuture<'_, ()> {
        Box::pin(async move {
            self.state = msg;
        })
    }
}

fn bench_statechart_memory_footprint(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_footprint");

    for num_states in &[10, 50, 100, 500] {
        group.bench_with_input(
            BenchmarkId::new("statechart_size", num_states),
            num_states,
            |b, &num_states| {
                let allocator = TrackingAllocator::new();

                b.iter(|| {
                    // Measure actual memory usage using tracking allocator
                    allocator.reset();

                    // Create multiple actors to simulate statechart memory usage
                    // Each actor represents a state with associated data
                    let actors: Vec<BenchActor> = (0..num_states)
                        .map(|_| BenchActor::new(32)) // 32 bytes per state
                        .collect();

                    let memory_used = allocator.allocated_bytes();
                    black_box((actors, memory_used));
                });
            },
        );
    }

    group.finish();
}

fn bench_mailbox_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("mailbox_memory");

    for capacity in &[8, 32, 128, 512] {
        group.bench_with_input(
            BenchmarkId::new("queue_capacity", capacity),
            capacity,
            |b, &capacity| {
                let allocator = TrackingAllocator::new();

                b.iter(|| {
                    // Measure actual mailbox memory usage
                    allocator.reset();

                    // Create real mailbox instances with the given capacity
                    let (outbox, inbox) = create_mailbox::<u32>(capacity);

                    // Fill the mailbox to measure actual memory usage
                    for i in 0..capacity {
                        let _ = outbox.try_send(u32::try_from(i).unwrap_or(u32::MAX));
                    }

                    let memory_used = allocator.allocated_bytes();
                    black_box((outbox, inbox, memory_used));
                });
            },
        );
    }

    group.finish();
}

fn bench_actor_system_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("actor_scaling");

    for num_actors in &[1, 10, 100] {
        group.bench_with_input(
            BenchmarkId::new("memory_per_actor", num_actors),
            num_actors,
            |b, &num_actors| {
                let allocator = TrackingAllocator::new();

                b.iter(|| {
                    // Measure memory usage with real actor instances
                    allocator.reset();

                    // Create real actors with typical state and behavior
                    let actors: Vec<BenchActor> = (0..num_actors)
                        .map(|_| BenchActor::new(64)) // 64 bytes per actor state
                        .collect();

                    // Create mailboxes for each actor to simulate full system
                    let mailboxes: Vec<_> =
                        (0..num_actors).map(|_| create_mailbox::<u32>(16)).collect();

                    let memory_used = allocator.allocated_bytes();
                    black_box((actors, mailboxes, memory_used));
                });
            },
        );
    }

    group.finish();
}

fn bench_zero_allocation_paths(c: &mut Criterion) {
    let mut group = c.benchmark_group("zero_allocation");
    let allocator = TrackingAllocator::new();

    group.bench_function("no_std_transition", |b| {
        b.iter(|| {
            // Verify zero-allocation state transitions
            allocator.reset();
            let initial_count = allocator.allocated_bytes();

            // Simulate a state transition that should not allocate
            let mut actor = BenchActor {
                state: 0,
                data: Vec::new(),
            };
            actor.state = 42; // Simple state change without allocation

            let final_count = allocator.allocated_bytes();
            let allocations = final_count.saturating_sub(initial_count);

            // Assert zero allocations occurred
            assert_eq!(
                allocations, 0,
                "State transition should not allocate memory"
            );
            black_box((actor, allocations));
        });
    });

    group.bench_function("static_mailbox_operation", |b| {
        // Pre-create mailbox outside the benchmark to avoid allocation during test
        let (outbox, mut inbox) = create_mailbox::<u32>(16);

        b.iter(|| {
            // Verify static mailbox operations don't allocate
            allocator.reset();
            let initial_count = allocator.allocated_bytes();

            // Perform mailbox operations that should not allocate
            let _ = outbox.try_send(42);
            let _ = inbox.try_recv();

            let final_count = allocator.allocated_bytes();
            let allocations = final_count.saturating_sub(initial_count);

            // Assert zero allocations occurred
            assert_eq!(
                allocations, 0,
                "Mailbox operations should not allocate memory"
            );
            black_box(allocations);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_statechart_memory_footprint,
    bench_mailbox_memory_usage,
    bench_actor_system_scaling,
    bench_zero_allocation_paths
);
criterion_main!(benches);
