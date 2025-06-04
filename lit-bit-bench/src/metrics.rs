//! Benchmark metrics collection and reporting

use parking_lot::RwLock;
use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

#[cfg(feature = "profile-alloc")]
use {
    backtrace::Backtrace,
    std::sync::atomic::AtomicBool,
    tracing::{info, warn},
};

/// Throughput metrics for benchmark runs
#[derive(Debug, Clone)]
pub struct ThroughputMetrics {
    pub messages_per_second: u64,
    pub total_messages: u64,
    pub duration: Duration,
}

/// Latency metrics for benchmark runs
#[derive(Debug, Clone)]
pub struct LatencyMetrics {
    pub p50: Duration,
    pub p95: Duration,
    pub p99: Duration,
    pub p999: Duration,
    pub min: Duration,
    pub max: Duration,
    pub mean: Duration,
}

/// Memory metrics for benchmark runs
#[derive(Debug, Clone)]
pub struct MemoryMetrics {
    pub bytes_per_actor: usize,
    pub peak_allocation: usize,
    pub allocation_count: usize,
    pub fragmentation_ratio: f64,
}

/// CPU metrics for benchmark runs
#[derive(Debug, Clone)]
pub struct CPUMetrics {
    pub instructions_per_cycle: f64,
    pub cache_miss_rate: f64,
    pub context_switches: u64,
}

/// Complete benchmark results
#[derive(Debug, Clone)]
pub struct BenchmarkResults {
    pub name: String,
    pub throughput: ThroughputMetrics,
    pub latency: LatencyMetrics,
    pub memory: MemoryMetrics,
    pub cpu: CPUMetrics,
}

/// Latency measurement meter
pub struct LatencyMeter {
    samples: Vec<Duration>,
    sorted: bool,
}

impl Default for LatencyMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl LatencyMeter {
    pub fn new() -> Self {
        Self {
            samples: Vec::with_capacity(1000),
            sorted: false,
        }
    }

    pub fn record(&mut self, duration: Duration) {
        self.samples.push(duration);
        self.sorted = false;
    }

    pub fn percentile(&mut self, p: f64) -> Duration {
        if !self.sorted {
            self.samples.sort_unstable();
            self.sorted = true;
        }

        if self.samples.is_empty() {
            return Duration::from_secs(0);
        }

        let idx = (self.samples.len() as f64 * p).ceil() as usize - 1;
        self.samples[idx.min(self.samples.len() - 1)]
    }

    pub fn mean(&self) -> Duration {
        if self.samples.is_empty() {
            return Duration::from_secs(0);
        }

        let sum: u128 = self.samples.iter().map(|d| d.as_nanos()).sum();
        Duration::from_nanos((sum / self.samples.len() as u128) as u64)
    }

    pub fn to_metrics(&mut self) -> LatencyMetrics {
        LatencyMetrics {
            p50: self.percentile(0.50),
            p95: self.percentile(0.95),
            p99: self.percentile(0.99),
            p999: self.percentile(0.999),
            min: self.samples.first().copied().unwrap_or_default(),
            max: self.samples.last().copied().unwrap_or_default(),
            mean: self.mean(),
        }
    }
}

/// Allocation tracking counter for zero-alloc validation
#[cfg(feature = "profile-alloc")]
pub static ALLOC_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Flag to track if we've seen any allocations
#[cfg(feature = "profile-alloc")]
static FIRST_ALLOC_SEEN: AtomicBool = AtomicBool::new(false);

/// Memory tracking allocator for benchmarks
#[derive(Debug)]
pub struct TrackingAllocator {
    inner: System,
    allocated: AtomicUsize,
    deallocated: AtomicUsize,
    peak: AtomicUsize,
    allocation_count: AtomicUsize,
}

impl TrackingAllocator {
    pub const fn new() -> Self {
        Self {
            inner: System,
            allocated: AtomicUsize::new(0),
            deallocated: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
            allocation_count: AtomicUsize::new(0),
        }
    }

    pub fn allocated_bytes(&self) -> usize {
        self.allocated.load(Ordering::Relaxed)
    }

    pub fn deallocated_bytes(&self) -> usize {
        self.deallocated.load(Ordering::Relaxed)
    }

    pub fn peak_allocation(&self) -> usize {
        self.peak.load(Ordering::Relaxed)
    }

    pub fn record_allocation(&self, size: usize) {
        let current = self.allocated.fetch_add(size, Ordering::Relaxed) + size;

        #[cfg(feature = "profile-alloc")]
        {
            let count = self.allocation_count.fetch_add(1, Ordering::Relaxed) + 1;
            ALLOC_COUNTER.fetch_add(1, Ordering::Relaxed);

            // Log first allocation with backtrace
            if !FIRST_ALLOC_SEEN.swap(true, Ordering::Relaxed) {
                let bt = Backtrace::new();
                warn!(
                    target: "alloc_trace",
                    "🚨 First allocation detected!\nSize: {} bytes\nTotal count: {}\nBacktrace:\n{:?}",
                    size, count, bt
                );
            }

            // Log allocation stats
            info!(
                target: "alloc_stats",
                "📊 Allocation #{count}: size={size}, total={current}, peak={}",
                self.peak.load(Ordering::Relaxed)
            );
        }

        #[cfg(not(feature = "profile-alloc"))]
        {
            self.allocation_count.fetch_add(1, Ordering::Relaxed);
        }

        // Update peak tracking
        let mut peak = self.peak.load(Ordering::Relaxed);
        while current > peak {
            match self.peak.compare_exchange_weak(
                peak,
                current,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(p) => peak = p,
            }
        }
    }

    pub fn record_deallocation(&self, size: usize) {
        self.deallocated.fetch_add(size, Ordering::Relaxed);
    }

    pub fn metrics(&self) -> MemoryMetrics {
        let allocated = self.allocated.load(Ordering::Relaxed);
        let deallocated = self.deallocated.load(Ordering::Relaxed);

        MemoryMetrics {
            bytes_per_actor: 0, // Needs to be set by benchmark
            peak_allocation: self.peak.load(Ordering::Relaxed),
            allocation_count: self.allocation_count.load(Ordering::Relaxed),
            fragmentation_ratio: if allocated == 0 {
                0.0
            } else {
                (allocated - deallocated) as f64 / allocated as f64
            },
        }
    }

    pub fn reset(&self) {
        self.allocated.store(0, Ordering::Relaxed);
        self.deallocated.store(0, Ordering::Relaxed);
        self.peak.store(0, Ordering::Relaxed);
        self.allocation_count.store(0, Ordering::Relaxed);

        #[cfg(feature = "profile-alloc")]
        {
            ALLOC_COUNTER.store(0, Ordering::Relaxed);
            FIRST_ALLOC_SEEN.store(false, Ordering::Relaxed);
            info!(target: "alloc_trace", "🔄 Allocation tracking reset");
        }
    }
}

impl Default for TrackingAllocator {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { self.inner.alloc(layout) };
        if !ptr.is_null() {
            self.record_allocation(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { self.inner.dealloc(ptr, layout) };
        self.record_deallocation(layout.size());
    }
}

/// Benchmark results dashboard for tracking and analysis
pub struct BenchmarkDashboard {
    results: RwLock<BTreeMap<String, Vec<BenchmarkResults>>>,
    regression_threshold: f64,
}

impl BenchmarkDashboard {
    pub fn new(regression_threshold: f64) -> Self {
        Self {
            results: RwLock::new(BTreeMap::new()),
            regression_threshold,
        }
    }

    pub fn record_result(&self, result: BenchmarkResults) {
        let mut results = self.results.write();
        results.entry(result.name.clone()).or_default().push(result);
    }

    pub fn detect_regressions(&self) -> Vec<Regression> {
        let results = self.results.read();
        let mut regressions = Vec::new();

        for (name, history) in results.iter() {
            if history.len() < 2 {
                continue;
            }

            let latest = &history[history.len() - 1];
            let previous = &history[history.len() - 2];

            // Check throughput regression
            let throughput_ratio = latest.throughput.messages_per_second as f64
                / previous.throughput.messages_per_second as f64;
            if throughput_ratio < (1.0 - self.regression_threshold) {
                regressions.push(Regression {
                    benchmark_name: name.clone(),
                    metric: "throughput".to_string(),
                    previous_value: previous.throughput.messages_per_second as f64,
                    current_value: latest.throughput.messages_per_second as f64,
                    regression_ratio: throughput_ratio,
                });
            }

            // Check latency regression
            let latency_ratio =
                latest.latency.p99.as_nanos() as f64 / previous.latency.p99.as_nanos() as f64;
            if latency_ratio > (1.0 + self.regression_threshold) {
                regressions.push(Regression {
                    benchmark_name: name.clone(),
                    metric: "p99_latency".to_string(),
                    previous_value: previous.latency.p99.as_nanos() as f64,
                    current_value: latest.latency.p99.as_nanos() as f64,
                    regression_ratio: latency_ratio,
                });
            }
        }

        regressions
    }

    pub fn generate_leaderboard(&self) -> String {
        let results = self.results.read();
        let mut output = String::from("🏆 Performance Leaderboard\n=======================\n\n");

        // Most Throughput
        results
            .iter()
            .max_by_key(|(_, h)| h.last().map(|r| r.throughput.messages_per_second))
            .and_then(|(name, history)| {
                history.last().map(|result| {
                    output.push_str(&format!(
                        "🥇 Highest Throughput: {} ({} msg/s)\n",
                        name, result.throughput.messages_per_second
                    ));
                })
            });

        // Best Latency
        results
            .iter()
            .min_by_key(|(_, h)| h.last().map(|r| r.latency.p99.as_nanos()))
            .and_then(|(name, history)| {
                history.last().map(|result| {
                    output.push_str(&format!(
                        "🥈 Best Latency: {} (p99: {}µs)\n",
                        name,
                        result.latency.p99.as_micros()
                    ));
                })
            });

        // Most Memory Efficient
        results
            .iter()
            .min_by_key(|(_, h)| h.last().map(|r| r.memory.bytes_per_actor))
            .and_then(|(name, history)| {
                history.last().map(|result| {
                    output.push_str(&format!(
                        "🥉 Most Memory Efficient: {} ({} bytes/actor)\n",
                        name, result.memory.bytes_per_actor
                    ));
                })
            });

        output
    }
}

#[derive(Debug)]
pub struct Regression {
    pub benchmark_name: String,
    pub metric: String,
    pub previous_value: f64,
    pub current_value: f64,
    pub regression_ratio: f64,
}

/// Snapshot of allocation metrics
#[derive(Debug, Clone, Copy)]
pub struct AllocationMetrics {
    pub allocated: usize,
    pub deallocated: usize,
    pub peak: usize,
    pub count: usize,
}

impl AllocationMetrics {
    /// Returns true if no allocations have occurred
    pub fn is_zero_alloc(&self) -> bool {
        self.count == 0
    }

    /// Format metrics for display
    pub fn format_stats(&self) -> String {
        format!(
            "📊 Allocation Stats:\n\
             Total allocated: {} bytes\n\
             Total deallocated: {} bytes\n\
             Peak usage: {} bytes\n\
             Allocation count: {}\n\
             Status: {}",
            self.allocated,
            self.deallocated,
            self.peak,
            self.count,
            if self.is_zero_alloc() {
                "✅ Zero allocations"
            } else {
                "⚠️ Allocations detected"
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_meter() {
        let mut meter = LatencyMeter::new();
        meter.record(Duration::from_nanos(100));
        meter.record(Duration::from_nanos(200));
        meter.record(Duration::from_nanos(300));
        meter.record(Duration::from_nanos(400));

        assert_eq!(meter.percentile(0.50), Duration::from_nanos(200));
        assert_eq!(meter.percentile(0.95), Duration::from_nanos(400));
        assert_eq!(meter.mean(), Duration::from_nanos(250));
    }

    #[test]
    fn test_bench_allocator() {
        let allocator = TrackingAllocator::new();
        allocator.record_allocation(100);
        allocator.record_allocation(50);
        allocator.record_deallocation(30);

        let metrics = allocator.metrics();
        assert_eq!(metrics.peak_allocation, 150);
        assert_eq!(metrics.allocation_count, 2);
        assert!((metrics.fragmentation_ratio - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_dashboard_regression_detection() {
        let dashboard = BenchmarkDashboard::new(0.1); // 10% threshold

        // Record initial result
        dashboard.record_result(BenchmarkResults {
            name: "test".to_string(),
            throughput: ThroughputMetrics {
                messages_per_second: 1_000_000,
                total_messages: 1_000_000,
                duration: Duration::from_secs(1),
            },
            latency: LatencyMetrics {
                p50: Duration::from_micros(100),
                p95: Duration::from_micros(200),
                p99: Duration::from_micros(300),
                p999: Duration::from_micros(400),
                min: Duration::from_micros(50),
                max: Duration::from_micros(500),
                mean: Duration::from_micros(150),
            },
            memory: MemoryMetrics {
                bytes_per_actor: 1024,
                peak_allocation: 10240,
                allocation_count: 10,
                fragmentation_ratio: 0.1,
            },
            cpu: CPUMetrics {
                instructions_per_cycle: 2.0,
                cache_miss_rate: 0.01,
                context_switches: 100,
            },
        });

        // Record regressed result
        dashboard.record_result(BenchmarkResults {
            name: "test".to_string(),
            throughput: ThroughputMetrics {
                messages_per_second: 800_000, // 20% regression
                total_messages: 800_000,
                duration: Duration::from_secs(1),
            },
            latency: LatencyMetrics {
                p50: Duration::from_micros(120),
                p95: Duration::from_micros(240),
                p99: Duration::from_micros(360), // 20% regression
                p999: Duration::from_micros(480),
                min: Duration::from_micros(60),
                max: Duration::from_micros(600),
                mean: Duration::from_micros(180),
            },
            memory: MemoryMetrics {
                bytes_per_actor: 1024,
                peak_allocation: 10240,
                allocation_count: 10,
                fragmentation_ratio: 0.1,
            },
            cpu: CPUMetrics {
                instructions_per_cycle: 1.8,
                cache_miss_rate: 0.012,
                context_switches: 120,
            },
        });

        let regressions = dashboard.detect_regressions();
        assert_eq!(regressions.len(), 2); // Both throughput and latency regressed

        // Check throughput regression
        let throughput_regression = regressions
            .iter()
            .find(|r| r.metric == "throughput")
            .unwrap();
        assert!((throughput_regression.regression_ratio - 0.8).abs() < f64::EPSILON);

        // Check latency regression
        let latency_regression = regressions
            .iter()
            .find(|r| r.metric == "p99_latency")
            .unwrap();
        assert!((latency_regression.regression_ratio - 1.2).abs() < f64::EPSILON);
    }
}
