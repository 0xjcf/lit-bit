use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use lit_bit_bench::{
    BenchmarkDashboard, RuntimeType, create_executor,
    metrics::{BenchmarkResults, CPUMetrics, LatencyMetrics, MemoryMetrics, ThroughputMetrics},
};
use lit_bit_core::actor::{Actor, create_mailbox};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Test message for throughput benchmarking
#[derive(Debug, Clone)]
struct TestMessage(u32);

/// Actor for throughput testing
struct ThroughputActor {
    count: u32,
}

impl ThroughputActor {
    fn new() -> Self {
        Self { count: 0 }
    }
}

impl Actor for ThroughputActor {
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

fn bench_message_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_throughput");
    let dashboard = Arc::new(BenchmarkDashboard::new(0.1)); // 10% regression threshold

    // Test different message batch sizes
    for &messages_per_batch in &[1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(messages_per_batch));

        // Test each runtime type
        for runtime_type in [
            RuntimeType::TokioSingleThread,
            RuntimeType::TokioMultiThread,
            RuntimeType::FuturesLite,
        ] {
            group.bench_with_input(
                BenchmarkId::new(
                    match runtime_type {
                        RuntimeType::TokioSingleThread => "tokio_single",
                        RuntimeType::TokioMultiThread => "tokio_multi",
                        RuntimeType::FuturesLite => "futures_lite",
                        #[cfg(feature = "runtime-embassy")]
                        RuntimeType::Embassy => "embassy",
                    },
                    messages_per_batch,
                ),
                &messages_per_batch,
                |b, &batch_size| {
                    let executor = create_executor(
                        runtime_type,
                        if matches!(runtime_type, RuntimeType::TokioMultiThread) {
                            Some(num_cpus::get())
                        } else {
                            None
                        },
                    );

                    b.iter(|| {
                        executor.block_on(async {
                            let mut actor = ThroughputActor::new();
                            let (outbox, mut inbox) =
                                create_mailbox(batch_size.try_into().unwrap());

                            // Send messages
                            let send_start = Instant::now();
                            for i in 0..batch_size {
                                outbox
                                    .try_send(TestMessage(i as u32))
                                    .expect("Failed to send message");
                            }
                            let send_duration = send_start.elapsed();

                            // Process messages
                            let process_start = Instant::now();
                            while let Ok(msg) = inbox.try_recv() {
                                std::mem::drop(actor.handle(msg));
                            }
                            let process_duration = process_start.elapsed();

                            // Create benchmark results
                            let results = BenchmarkResults {
                                name: format!("{runtime_type:?}_{batch_size}"),
                                throughput: ThroughputMetrics {
                                    messages_per_second: (batch_size * 1_000_000_000)
                                        / (send_duration + process_duration).as_nanos() as u64,
                                    total_messages: batch_size,
                                    duration: send_duration + process_duration,
                                },
                                latency: LatencyMetrics {
                                    p50: Duration::from_nanos(0), // Not measured in throughput test
                                    p95: Duration::from_nanos(0),
                                    p99: Duration::from_nanos(0),
                                    p999: Duration::from_nanos(0),
                                    min: Duration::from_nanos(0),
                                    max: Duration::from_nanos(0),
                                    mean: Duration::from_nanos(0),
                                },
                                memory: MemoryMetrics {
                                    bytes_per_actor: std::mem::size_of::<ThroughputActor>(),
                                    peak_allocation: 0, // Not tracked in this test
                                    allocation_count: 0,
                                    fragmentation_ratio: 0.0,
                                },
                                cpu: CPUMetrics {
                                    instructions_per_cycle: 0.0,
                                    cache_miss_rate: 0.0,
                                    context_switches: 0,
                                },
                            };

                            dashboard.record_result(results);
                        });
                    });
                },
            );
        }
    }

    // Generate and print the leaderboard
    println!("\n{}", dashboard.generate_leaderboard());

    // Check for regressions
    let regressions = dashboard.detect_regressions();
    if !regressions.is_empty() {
        println!("\n⚠️ Performance Regressions Detected:");
        for regression in regressions {
            println!(
                "- {}: {} regressed by {:.1}% ({:.2} -> {:.2})",
                regression.benchmark_name,
                regression.metric,
                (1.0 - regression.regression_ratio) * 100.0,
                regression.previous_value,
                regression.current_value,
            );
        }
    }

    group.finish();
}

criterion_group!(benches, bench_message_throughput);
criterion_main!(benches);
