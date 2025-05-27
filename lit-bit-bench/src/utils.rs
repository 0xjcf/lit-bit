//! Benchmark utilities for measuring performance and memory usage

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// A simple allocator wrapper to track memory allocations
pub struct TrackingAllocator {
    inner: System,
    allocated: AtomicUsize,
    deallocated: AtomicUsize,
}

impl TrackingAllocator {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: System,
            allocated: AtomicUsize::new(0),
            deallocated: AtomicUsize::new(0),
        }
    }

    pub fn allocated_bytes(&self) -> usize {
        self.allocated.load(Ordering::Relaxed)
    }

    pub fn deallocated_bytes(&self) -> usize {
        self.deallocated.load(Ordering::Relaxed)
    }

    #[allow(clippy::cast_possible_wrap)]
    pub fn net_allocated(&self) -> isize {
        self.allocated_bytes() as isize - self.deallocated_bytes() as isize
    }

    pub fn reset(&self) {
        self.allocated.store(0, Ordering::Relaxed);
        self.deallocated.store(0, Ordering::Relaxed);
    }
}

impl Default for TrackingAllocator {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: We're delegating to the system allocator
        let ptr = unsafe { self.inner.alloc(layout) };
        if !ptr.is_null() {
            self.allocated.fetch_add(layout.size(), Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: We're delegating to the system allocator with the same ptr and layout
        unsafe { self.inner.dealloc(ptr, layout) };
        self.deallocated.fetch_add(layout.size(), Ordering::Relaxed);
    }
}

/// Measure the execution time of a closure
pub fn measure_time<F, R>(f: F) -> (R, Duration)
where
    F: FnOnce() -> R,
{
    let start = Instant::now();
    let result = f();
    let duration = start.elapsed();
    (result, duration)
}

/// Measure memory allocation during closure execution
pub fn measure_allocation<F, R>(allocator: &TrackingAllocator, f: F) -> (R, usize)
where
    F: FnOnce() -> R,
{
    allocator.reset();
    let result = f();
    let allocated = allocator.allocated_bytes();
    (result, allocated)
}

/// Benchmark configuration for consistent testing
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub warmup_iterations: usize,
    pub measurement_iterations: usize,
    pub sample_size: usize,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            warmup_iterations: 10,
            measurement_iterations: 100,
            sample_size: 1000,
        }
    }
}

/// Run a benchmark with the given configuration
pub fn run_benchmark<F>(config: &BenchmarkConfig, mut f: F) -> BenchmarkResults
where
    F: FnMut(),
{
    // Warmup
    for _ in 0..config.warmup_iterations {
        f();
    }

    // Measurement
    let mut durations = Vec::with_capacity(config.measurement_iterations);
    for _ in 0..config.measurement_iterations {
        let ((), duration) = measure_time(&mut f);
        durations.push(duration);
    }

    BenchmarkResults::from_durations(durations)
}

/// Results from a benchmark run
#[derive(Debug, Clone)]
pub struct BenchmarkResults {
    pub mean: Duration,
    pub median: Duration,
    pub min: Duration,
    pub max: Duration,
    pub std_dev: Duration,
}

impl BenchmarkResults {
    /// Create benchmark results from a vector of durations
    ///
    /// # Panics
    ///
    /// Panics if the number of durations is larger than `u32::MAX`.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    pub fn from_durations(mut durations: Vec<Duration>) -> Self {
        durations.sort();

        let len = durations.len();
        let mean = durations.iter().sum::<Duration>() / u32::try_from(len).unwrap();
        let median = durations[len / 2];
        let min = durations[0];
        let max = durations[len - 1];

        // Calculate standard deviation
        let variance: f64 = durations
            .iter()
            .map(|d| {
                let diff = d.as_nanos() as f64 - mean.as_nanos() as f64;
                diff * diff
            })
            .sum::<f64>()
            / len as f64;

        let std_dev = Duration::from_nanos(variance.sqrt() as u64);

        Self {
            mean,
            median,
            min,
            max,
            std_dev,
        }
    }
}
