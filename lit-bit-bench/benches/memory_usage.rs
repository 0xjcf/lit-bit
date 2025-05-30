//! Memory usage benchmarks for statechart and actor systems

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use lit_bit_bench::utils::TrackingAllocator;
use lit_bit_core::actor::{Actor, create_mailbox};
use lit_bit_macro::statechart;

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
    type Future<'a>
        = core::future::Ready<()>
    where
        Self: 'a;

    fn handle(&mut self, msg: Self::Message) -> Self::Future<'_> {
        self.state = msg;
        core::future::ready(())
    }
}

// Statechart for memory benchmarks
#[derive(Debug, Clone, Default)]
pub struct MemoryBenchContext {
    counter: u32,
    operations: u32,
}

impl MemoryBenchContext {
    fn increment(&mut self) {
        self.counter += 1;
        self.operations += 1;
    }

    fn reset(&mut self) {
        self.counter = 0;
        self.operations += 1;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MemoryBenchEvent {
    #[default]
    Increment,
    Reset,
    Toggle,
}

fn action_increment(ctx: &mut MemoryBenchContext, _event: &MemoryBenchEvent) {
    ctx.increment();
}

fn action_reset(ctx: &mut MemoryBenchContext, _event: &MemoryBenchEvent) {
    ctx.reset();
}

statechart! {
    name: MemoryBenchMachine,
    context: MemoryBenchContext,
    event: MemoryBenchEvent,
    initial: StateA,

    state StateA {
        on MemoryBenchEvent::Increment => StateB [action action_increment];
        on MemoryBenchEvent::Reset => StateA [action action_reset];
    }

    state StateB {
        on MemoryBenchEvent::Toggle => StateA;
        on MemoryBenchEvent::Reset => StateA [action action_reset];
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

                    // Create multiple statecharts to simulate memory usage
                    let statecharts: Vec<_> = (0..num_states)
                        .map(|_| {
                            MemoryBenchMachine::new(
                                MemoryBenchContext::default(),
                                &MemoryBenchEvent::default(),
                            )
                            .expect("Failed to create statechart")
                        })
                        .collect();

                    let memory_used = allocator.allocated_bytes();
                    black_box((statecharts, memory_used));
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

            // Create and exercise a statechart that should not allocate
            let mut machine = MemoryBenchMachine::new(
                MemoryBenchContext::default(),
                &MemoryBenchEvent::default(),
            )
            .expect("Failed to create machine");

            let initial_count = allocator.allocated_bytes();

            // Perform transitions that should not allocate
            let result1 = machine.send(&MemoryBenchEvent::Increment);
            let result2 = machine.send(&MemoryBenchEvent::Toggle);
            let result3 = machine.send(&MemoryBenchEvent::Reset);

            let final_count = allocator.allocated_bytes();
            let allocations = final_count.saturating_sub(initial_count);

            // Assert zero allocations occurred during transitions
            assert_eq!(
                allocations, 0,
                "State transitions should not allocate memory during execution"
            );
            black_box((machine, result1, result2, result3, allocations));
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
            let send_result = outbox.try_send(42);
            let recv_result = inbox.try_recv();

            let final_count = allocator.allocated_bytes();
            let allocations = final_count.saturating_sub(initial_count);

            // Assert zero allocations occurred
            assert_eq!(
                allocations, 0,
                "Mailbox operations should not allocate memory"
            );
            let _ = black_box((send_result, recv_result, allocations));
        });
    });

    group.finish();
}

fn bench_kpi_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("kpi_validation");

    // Validate the 512B RAM overhead target for single actor with N=8 queue
    group.bench_function("single_actor_512b_target", |b| {
        let allocator = TrackingAllocator::new();
        let mut final_memory_used = 0;

        b.iter(|| {
            allocator.reset();

            // Create a single actor with 8-capacity mailbox
            let actor = BenchActor::new(64); // Reasonable state size
            let (outbox, inbox) = create_mailbox::<u32>(8);

            // Fill the mailbox to test actual usage
            for i in 0..8 {
                let _ = outbox.try_send(i);
            }

            let memory_used = allocator.allocated_bytes();
            final_memory_used = memory_used; // Capture for reporting later

            // In real embedded systems, this should be ≤ 512B
            // Here we just measure and report
            black_box((actor, outbox, inbox, memory_used));
        });

        // Report memory usage once after benchmark completes
        println!(
            "Single actor + N=8 mailbox memory usage: {} bytes (target: ≤512B)",
            final_memory_used
        );
    });

    group.finish();
}

fn bench_async_vs_sync_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("async_vs_sync_overhead");

    // Compare sync statechart performance
    group.bench_function("sync_statechart_transitions", |b| {
        b.iter(|| {
            let mut machine = MemoryBenchMachine::new(
                MemoryBenchContext::default(),
                &MemoryBenchEvent::default(),
            )
            .expect("Failed to create machine");

            // Perform 1000 sync transitions
            for i in 0..1000 {
                let event = if i % 3 == 0 {
                    MemoryBenchEvent::Reset
                } else {
                    MemoryBenchEvent::Increment
                };

                let result = machine.send(&event);
                black_box(result);
            }

            black_box(machine.context().operations);
        });
    });

    // Compare sync actor performance
    group.bench_function("sync_actor_messages", |b| {
        b.iter(|| {
            let (outbox, mut inbox) = create_mailbox::<u32>(1024);
            let mut actor = BenchActor::new(64);

            // Send and process 1000 messages synchronously
            for i in 0..1000 {
                outbox.try_send(i).unwrap_or_else(|_| {
                    // Drain if full
                    while let Ok(msg) = inbox.try_recv() {
                        let future = actor.handle(msg);
                        std::mem::drop(black_box(future));
                    }
                });
            }

            // Process remaining messages
            while let Ok(msg) = inbox.try_recv() {
                let future = actor.handle(msg);
                std::mem::drop(black_box(future));
            }

            black_box(actor.state);
        });
    });

    group.finish();
}

fn bench_memory_scaling_characteristics(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_scaling");

    // Test memory scaling with different mailbox sizes
    for mailbox_size in &[8, 16, 32, 64, 128] {
        group.bench_with_input(
            BenchmarkId::new("mailbox_scaling", mailbox_size),
            mailbox_size,
            |b, &size| {
                let allocator = TrackingAllocator::new();

                b.iter(|| {
                    allocator.reset();

                    let actor = BenchActor::new(32);
                    let (outbox, _inbox) = create_mailbox::<u32>(size);

                    let memory_used = allocator.allocated_bytes();
                    let memory_per_slot = if size > 0 { memory_used / size } else { 0 };

                    black_box((actor, outbox, memory_used, memory_per_slot));
                });
            },
        );
    }

    // Test memory scaling with different numbers of states
    for num_states in &[1, 5, 10, 20, 50] {
        group.bench_with_input(
            BenchmarkId::new("statechart_scaling", num_states),
            num_states,
            |b, &num_states| {
                let allocator = TrackingAllocator::new();

                b.iter(|| {
                    allocator.reset();

                    let statecharts: Vec<_> = (0..num_states)
                        .map(|_| {
                            MemoryBenchMachine::new(
                                MemoryBenchContext::default(),
                                &MemoryBenchEvent::default(),
                            )
                            .expect("Failed to create statechart")
                        })
                        .collect();

                    let memory_used = allocator.allocated_bytes();
                    let memory_per_statechart = if num_states > 0 {
                        memory_used / num_states
                    } else {
                        0
                    };

                    black_box((statecharts, memory_used, memory_per_statechart));
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_statechart_memory_footprint,
    bench_mailbox_memory_usage,
    bench_actor_system_scaling,
    bench_zero_allocation_paths,
    bench_kpi_validation,
    bench_async_vs_sync_overhead,
    bench_memory_scaling_characteristics
);
criterion_main!(benches);
