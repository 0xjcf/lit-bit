//! Performance benchmarks for lit-bit
//!
//! This crate contains Criterion.rs benchmarks for measuring performance across
//! different runtimes and environments, with support for both host and embedded targets.

pub mod fixtures;
pub mod metrics;
pub mod runtime;

// Re-export commonly used types
pub use metrics::{
    BenchmarkDashboard, BenchmarkResults, CPUMetrics, LatencyMeter, LatencyMetrics, MemoryMetrics,
    ThroughputMetrics, TrackingAllocator,
};
pub use runtime::{BenchExecutor, RuntimeType, create_executor};

use std::os::unix::process::ExitStatusExt;
use std::process::{Command, ExitStatus}; // For Unix-like systems

/// Common benchmark utilities and test data
pub mod common {
    use super::*;

    /// Common events for benchmark testing
    #[derive(Debug, Clone, PartialEq)]
    pub enum BenchEvent {
        Transition(u32),
        Batch(Vec<u32>),
        Reset,
    }

    /// Performance test configuration
    #[derive(Debug, Clone)]
    pub struct BenchConfig {
        pub runtime: RuntimeType,
        pub worker_threads: Option<usize>,
        pub warmup_iterations: usize,
        pub measurement_iterations: usize,
        pub sample_size: usize,
    }

    impl Default for BenchConfig {
        fn default() -> Self {
            Self {
                runtime: RuntimeType::TokioSingleThread,
                worker_threads: None,
                warmup_iterations: 10,
                measurement_iterations: 100,
                sample_size: 1000,
            }
        }
    }

    pub fn collect_perf_stats() -> CPUMetrics {
        // Run perf stat to collect CPU metrics
        let output = Command::new("perf")
            .args([
                "stat",
                "-e",
                "instructions,cycles,cache-misses,context-switches",
                "-x",
                ",", // CSV output
                "sleep",
                "1", // Measure for 1 second
            ])
            .output()
            .unwrap_or_else(|_| {
                println!("⚠️  perf stat not available - using dummy metrics");
                std::process::Output {
                    status: ExitStatus::from_raw(0),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                }
            });

        // Parse perf stat output
        let stats = String::from_utf8_lossy(&output.stderr);
        let mut instructions = 0.0;
        let mut cycles = 0.0;
        let mut cache_misses = 0.0;
        let mut context_switches = 0;

        for line in stats.lines() {
            let fields: Vec<&str> = line.split(',').collect();
            if fields.len() >= 3 {
                match fields[2].trim() {
                    "instructions" => instructions = fields[0].trim().parse().unwrap_or(0.0),
                    "cycles" => cycles = fields[0].trim().parse().unwrap_or(0.0),
                    "cache-misses" => cache_misses = fields[0].trim().parse().unwrap_or(0.0),
                    "context-switches" => context_switches = fields[0].trim().parse().unwrap_or(0),
                    _ => {}
                }
            }
        }

        CPUMetrics {
            instructions_per_cycle: if cycles > 0.0 {
                instructions / cycles
            } else {
                0.0
            },
            cache_miss_rate: if instructions > 0.0 {
                cache_misses / instructions
            } else {
                0.0
            },
            context_switches: context_switches as u64,
        }
    }

    /// Run a benchmark with the given configuration
    pub fn run_benchmark<F>(config: &BenchConfig, mut f: F) -> BenchmarkResults
    where
        F: FnMut(),
    {
        let executor = create_executor(config.runtime, config.worker_threads);
        let mut latency_meter = LatencyMeter::new();
        let allocator = TrackingAllocator::new();

        // Warmup
        for _ in 0..config.warmup_iterations {
            executor.block_on(async {
                f();
            });
        }

        // Collect CPU metrics during measurement
        let cpu_metrics = collect_perf_stats();

        // Measurement
        let start = std::time::Instant::now();
        for _ in 0..config.measurement_iterations {
            allocator.reset();
            let iteration_start = std::time::Instant::now();
            executor.block_on(async {
                f();
            });
            latency_meter.record(iteration_start.elapsed());
        }
        let duration = start.elapsed();

        BenchmarkResults {
            name: "benchmark".to_string(),
            throughput: ThroughputMetrics {
                messages_per_second: (config.sample_size as u64
                    * config.measurement_iterations as u64)
                    / duration.as_secs(),
                total_messages: config.sample_size as u64 * config.measurement_iterations as u64,
                duration,
            },
            latency: latency_meter.to_metrics(),
            memory: allocator.metrics(),
            cpu: cpu_metrics,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_benchmark_config() {
        let config = common::BenchConfig::default();
        assert_eq!(config.runtime, RuntimeType::TokioSingleThread);
        assert_eq!(config.warmup_iterations, 10);
        assert_eq!(config.measurement_iterations, 100);
    }

    #[test]
    fn test_run_benchmark() {
        let config = common::BenchConfig {
            sample_size: 10,
            measurement_iterations: 5,
            ..Default::default()
        };

        let results = common::run_benchmark(&config, || {
            std::thread::sleep(Duration::from_millis(1));
        });

        assert_eq!(results.throughput.total_messages, 50); // 10 samples * 5 iterations
        assert!(results.latency.p99 >= Duration::from_millis(1));
    }
}
