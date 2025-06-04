use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lit_bit_core::actor::{Actor, create_mailbox};
use std::time::Instant;
use tokio::runtime::Builder as RuntimeBuilder;

#[derive(Debug)]
struct ScalingActor {
    count: u64,
}

impl ScalingActor {
    fn new() -> Self {
        Self { count: 0 }
    }
}

#[derive(Debug)]
enum ScalingMessage {
    Increment,
    GetCount,
}

impl Actor for ScalingActor {
    type Message = ScalingMessage;
    type Future<'a>
        = std::future::Ready<()>
    where
        Self: 'a;

    fn handle(&mut self, msg: Self::Message) -> Self::Future<'_> {
        match msg {
            ScalingMessage::Increment => {
                self.count += 1;
            }
            ScalingMessage::GetCount => {
                // Just read the count
            }
        }
        std::future::ready(())
    }
}

fn bench_multi_thread_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_thread_scaling");

    // Test with different thread counts
    for threads in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("threads", threads),
            threads,
            |b, &thread_count| {
                b.iter_custom(|iters| {
                    // Create multi-threaded runtime with specified threads
                    let rt = RuntimeBuilder::new_multi_thread()
                        .worker_threads(thread_count)
                        .enable_all()
                        .build()
                        .unwrap();

                    // Create actors (one per thread)
                    let actors: Vec<_> = (0..thread_count)
                        .map(|_| {
                            let actor = ScalingActor::new();
                            let (outbox, _inbox) = create_mailbox(1000);
                            (actor, outbox)
                        })
                        .collect();

                    let start = Instant::now();

                    // Run the benchmark
                    rt.block_on(async {
                        let mut handles = Vec::new();

                        // Spawn tasks to send messages to each actor
                        for (_actor, outbox) in actors {
                            // Prefix unused variable with underscore
                            let handle = tokio::spawn(async move {
                                let messages_per_actor = iters / thread_count as u64;
                                for _ in 0..messages_per_actor {
                                    let _ = outbox.try_send(ScalingMessage::Increment);
                                }
                                // Final count check
                                let _ = outbox.try_send(ScalingMessage::GetCount);
                            });
                            handles.push(handle);
                        }

                        // Wait for all tasks to complete
                        for handle in handles {
                            let _ = handle.await;
                        }
                    });

                    let duration = start.elapsed();

                    // Calculate messages per second
                    let msgs_per_sec = (iters as f64 / duration.as_secs_f64()) as u64;

                    // Print scaling efficiency
                    let baseline = if thread_count == 1 {
                        msgs_per_sec // This is our baseline
                    } else {
                        msgs_per_sec / thread_count as u64 // Scale linearly
                    };

                    let efficiency = (msgs_per_sec as f64 / baseline as f64) * 100.0;

                    println!("\n📊 Thread Scaling Results (threads: {thread_count}):");
                    println!("=====================================");
                    println!("Messages/sec: {msgs_per_sec}");
                    if thread_count > 1 {
                        println!("Scaling efficiency: {efficiency:.1}%");
                        println!("KPI target: >80% scaling efficiency");
                        println!(
                            "Status: {}",
                            if efficiency >= 80.0 {
                                "✅ PASS"
                            } else {
                                "❌ FAIL"
                            }
                        );
                    }

                    duration
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_multi_thread_scaling);
criterion_main!(benches);
