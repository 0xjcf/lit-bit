//! Latency benchmarks for actor mailbox operations

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use futures::executor;
use lit_bit_core::actor::{Actor, create_mailbox};
use std::time::Instant;

// Test actor for latency benchmarks
#[derive(Debug)]
struct LatencyTestActor {
    messages_processed: u64,
}

impl LatencyTestActor {
    fn new() -> Self {
        Self {
            messages_processed: 0,
        }
    }
}

impl Actor for LatencyTestActor {
    type Message = LatencyTestMessage;
    type Future<'a>
        = core::future::Ready<()>
    where
        Self: 'a;

    fn handle(&mut self, msg: Self::Message) -> Self::Future<'_> {
        // Clean message processing without timing overhead for accurate benchmarks
        match msg {
            LatencyTestMessage::Increment => {
                self.messages_processed += 1;
            }
            LatencyTestMessage::Reset => {
                self.messages_processed = 0;
            }
            LatencyTestMessage::Ping => {
                // Simple ping message for latency testing
                black_box(self.messages_processed);
            }
        }

        core::future::ready(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LatencyTestMessage {
    Increment,
    Reset,
    Ping,
}

fn bench_mailbox_send_receive(c: &mut Criterion) {
    let mut group = c.benchmark_group("mailbox_latency");

    for queue_size in &[8, 32, 128] {
        group.bench_with_input(
            BenchmarkId::new("send_receive", queue_size),
            queue_size,
            |b, &queue_size| {
                b.iter(|| {
                    let (outbox, mut inbox) = create_mailbox::<LatencyTestMessage>(queue_size);
                    let mut actor = LatencyTestActor::new();

                    // Fill the mailbox partway to test realistic conditions
                    let fill_count = queue_size / 2;
                    for i in 0..fill_count {
                        let msg = if i % 2 == 0 {
                            LatencyTestMessage::Increment
                        } else {
                            LatencyTestMessage::Ping
                        };
                        outbox.try_send(msg).expect("Failed to send message");
                    }

                    // Measure send/receive latency
                    let start = Instant::now();

                    // Process all messages
                    while let Ok(msg) = inbox.try_recv() {
                        let future = actor.handle(msg);
                        executor::block_on(future); // Properly poll the future to completion
                    }

                    let elapsed = start.elapsed();
                    black_box((actor.messages_processed, elapsed));
                });
            },
        );
    }

    group.finish();
}

fn bench_mailbox_backpressure(c: &mut Criterion) {
    let mut group = c.benchmark_group("mailbox_backpressure");

    group.bench_function("full_queue_handling", |b| {
        b.iter(|| {
            // Test backpressure handling with a small queue
            let (outbox, mut inbox) = create_mailbox::<LatencyTestMessage>(8);
            let mut actor = LatencyTestActor::new();

            // Fill the queue completely
            for i in 0..8 {
                let msg = if i % 2 == 0 {
                    LatencyTestMessage::Increment
                } else {
                    LatencyTestMessage::Ping
                };
                outbox.try_send(msg).expect("Failed to fill queue");
            }

            // Try to send one more - this should fail due to backpressure
            let backpressure_result = outbox.try_send(LatencyTestMessage::Ping);
            assert!(backpressure_result.is_err(), "Expected backpressure error");

            // Process messages to free up space
            while let Ok(msg) = inbox.try_recv() {
                let future = actor.handle(msg);
                executor::block_on(future); // Properly poll the future to completion
            }

            // Now we should be able to send again
            let success_result = outbox.try_send(LatencyTestMessage::Ping);
            assert!(
                success_result.is_ok(),
                "Should be able to send after draining"
            );

            black_box(actor.messages_processed);
        });
    });

    group.finish();
}

fn bench_actor_spawn_time(c: &mut Criterion) {
    let mut group = c.benchmark_group("actor_spawn");

    group.bench_function("spawn_latency", |b| {
        b.iter(|| {
            // Measure the time to create an actor and mailbox
            let start = Instant::now();

            let actor = LatencyTestActor::new();
            let (outbox, _inbox) = create_mailbox::<LatencyTestMessage>(16);

            let spawn_time = start.elapsed();

            black_box((actor, outbox, spawn_time));
        });
    });

    group.finish();
}

fn bench_throughput_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_validation");

    // Validate we can process messages at target rate
    group.throughput(Throughput::Elements(100_000));
    group.bench_function("high_throughput_processing", |b| {
        b.iter(|| {
            let (outbox, mut inbox) = create_mailbox::<LatencyTestMessage>(1024);
            let mut actor = LatencyTestActor::new();

            // Send many messages
            for i in 0..100_000 {
                let msg = match i % 3 {
                    0 => LatencyTestMessage::Increment,
                    1 => LatencyTestMessage::Ping,
                    _ => LatencyTestMessage::Reset,
                };

                // If mailbox is full, drain it
                if outbox.try_send(msg).is_err() {
                    while let Ok(received_msg) = inbox.try_recv() {
                        let future = actor.handle(received_msg);
                        executor::block_on(future); // Properly poll the future to completion
                    }
                    // Try sending again
                    outbox.try_send(msg).expect("Should succeed after draining");
                }
            }

            // Process remaining messages
            while let Ok(msg) = inbox.try_recv() {
                let future = actor.handle(msg);
                executor::block_on(future); // Properly poll the future to completion
            }

            black_box(actor.messages_processed);
        });
    });

    group.finish();
}

fn bench_memory_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_efficiency");

    for num_actors in &[1, 10, 100] {
        group.bench_with_input(
            BenchmarkId::new("actors_memory_scaling", num_actors),
            num_actors,
            |b, &num_actors| {
                b.iter(|| {
                    // Create multiple actors to test memory scaling
                    let actors_and_mailboxes: Vec<_> = (0..num_actors)
                        .map(|_| {
                            let actor = LatencyTestActor::new();
                            let (outbox, inbox) = create_mailbox::<LatencyTestMessage>(16);
                            (actor, outbox, inbox)
                        })
                        .collect();

                    // Send a message to each actor to ensure they're exercised
                    for (mut actor, outbox, mut inbox) in actors_and_mailboxes {
                        outbox.try_send(LatencyTestMessage::Ping).unwrap();
                        if let Ok(msg) = inbox.try_recv() {
                            let future = actor.handle(msg);
                            executor::block_on(future); // Properly poll the future to completion
                        }
                        black_box((actor, outbox, inbox));
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_latency_targets(c: &mut Criterion) {
    let mut group = c.benchmark_group("latency_targets");

    // Test individual message latency to validate <100ns target
    group.bench_function("single_message_latency", |b| {
        let (outbox, mut inbox) = create_mailbox::<LatencyTestMessage>(16);
        let mut actor = LatencyTestActor::new();

        b.iter(|| {
            // Measure end-to-end latency for a single message
            let start = Instant::now();

            outbox
                .try_send(LatencyTestMessage::Ping)
                .expect("Failed to send");
            let msg = inbox.try_recv().expect("Failed to receive");
            let future = actor.handle(msg);
            executor::block_on(future); // Properly poll the future to completion

            let latency = start.elapsed();
            black_box(latency);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_mailbox_send_receive,
    bench_mailbox_backpressure,
    bench_actor_spawn_time,
    bench_throughput_validation,
    bench_memory_efficiency,
    bench_latency_targets
);
criterion_main!(benches);

// Iai-Callgrind benchmarks for instruction-level analysis
#[cfg(feature = "iai")]
mod iai_benches {
    use super::*;
    use iai_callgrind::{library_benchmark, library_benchmark_group, main};

    #[library_benchmark]
    fn iai_mailbox_send() {
        let (outbox, _inbox) = create_mailbox::<LatencyTestMessage>(16);
        let result = outbox.try_send(LatencyTestMessage::Ping);
        black_box(result);
    }

    #[library_benchmark]
    fn iai_actor_handle() {
        let mut actor = LatencyTestActor::new();
        let future = actor.handle(LatencyTestMessage::Increment);
        executor::block_on(future); // Properly poll the future to completion
    }

    #[library_benchmark]
    fn iai_statechart_transition() {
        // Use a simple counter to simulate state transition cost
        let mut counter = 0u64;
        counter += 1;
        black_box(counter);
    }

    library_benchmark_group!(
        name = iai_group;
        benchmarks = iai_mailbox_send, iai_actor_handle, iai_statechart_transition
    );

    main!(library_benchmark_groups = iai_group);
}
