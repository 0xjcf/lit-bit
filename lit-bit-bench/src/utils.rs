//! Benchmark utilities for measuring performance and memory usage

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// A simple allocator wrapper to track memory allocations
pub struct TrackingAllocator {
    inner: System,
    allocated: AtomicUsize,
    deallocated: AtomicUsize,
    peak: AtomicUsize, // Track peak allocation
}

impl TrackingAllocator {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: System,
            allocated: AtomicUsize::new(0),
            deallocated: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
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

    #[allow(clippy::cast_possible_wrap)]
    pub fn net_allocated(&self) -> isize {
        self.allocated_bytes() as isize - self.deallocated_bytes() as isize
    }

    pub fn reset(&self) {
        self.allocated.store(0, Ordering::Relaxed);
        self.deallocated.store(0, Ordering::Relaxed);
        self.peak.store(0, Ordering::Relaxed);
    }
}

impl Default for TrackingAllocator {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe {
            // SAFETY: We're delegating to the system allocator
            self.inner.alloc(layout)
        };
        if !ptr.is_null() {
            let current =
                self.allocated.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();

            // Update peak if current allocation exceeds it
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
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            // SAFETY: We're delegating to the system allocator with the same ptr and layout
            self.inner.dealloc(ptr, layout)
        };
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
    /// Panics if the input vector is empty or if the number of durations is larger than `u32::MAX`.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    pub fn from_durations(mut durations: Vec<Duration>) -> Self {
        assert!(
            !durations.is_empty(),
            "Cannot calculate statistics for empty duration vector"
        );

        durations.sort();

        let len = durations.len();

        // Calculate median correctly for both odd and even lengths
        let median = if len % 2 == 1 {
            durations[len / 2]
        } else {
            let mid1 = durations[len / 2 - 1];
            let mid2 = durations[len / 2];
            let avg_nanos = (mid1.as_nanos() + mid2.as_nanos()) / 2;
            Duration::from_nanos(u64::try_from(avg_nanos).unwrap_or(u64::MAX))
        };

        let min = durations[0];
        let max = durations[len - 1];

        // Calculate mean and standard deviation using Welford's method for numerical stability
        let mut mean_nanos = 0.0;
        let mut variance_accumulator = 0.0;

        for (i, duration) in durations.iter().enumerate() {
            let duration_nanos = duration.as_nanos() as f64;
            let delta = duration_nanos - mean_nanos;
            mean_nanos += delta / (i + 1) as f64;
            let delta2 = duration_nanos - mean_nanos;
            variance_accumulator += delta * delta2;
        }

        let mean = Duration::from_nanos(mean_nanos as u64);
        let variance = if len > 1 {
            variance_accumulator / len as f64
        } else {
            0.0
        };
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_median_odd_length() {
        let durations = vec![
            Duration::from_millis(1),
            Duration::from_millis(3),
            Duration::from_millis(2),
            Duration::from_millis(5),
            Duration::from_millis(4),
        ];
        let results = BenchmarkResults::from_durations(durations);
        assert_eq!(results.median, Duration::from_millis(3));
    }

    #[test]
    fn test_median_even_length() {
        let durations = vec![
            Duration::from_millis(1),
            Duration::from_millis(2),
            Duration::from_millis(3),
            Duration::from_millis(4),
        ];
        let results = BenchmarkResults::from_durations(durations);
        // Median should be (2 + 3) / 2 = 2.5ms = 2,500,000ns
        assert_eq!(results.median, Duration::from_nanos(2_500_000));
    }

    #[test]
    fn test_single_element() {
        let durations = vec![Duration::from_millis(5)];
        let results = BenchmarkResults::from_durations(durations);
        assert_eq!(results.median, Duration::from_millis(5));
        assert_eq!(results.mean, Duration::from_millis(5));
        assert_eq!(results.min, Duration::from_millis(5));
        assert_eq!(results.max, Duration::from_millis(5));
        assert_eq!(results.std_dev, Duration::from_nanos(0));
    }

    #[test]
    #[should_panic(expected = "Cannot calculate statistics for empty duration vector")]
    fn test_empty_vector_panics() {
        let durations = vec![];
        let _ = BenchmarkResults::from_durations(durations);
    }

    #[test]
    fn test_welford_method_accuracy() {
        // Test with values that have a clear standard deviation
        let durations = vec![
            Duration::from_millis(10),
            Duration::from_millis(20),
            Duration::from_millis(30),
        ];
        let results = BenchmarkResults::from_durations(durations);

        // Mean should be 20ms
        assert_eq!(results.mean, Duration::from_millis(20));

        // For values [10, 20, 30] with mean 20:
        // Variance = ((10)² + (0)² + (10)²) / 3 = 200/3 ≈ 66.67
        // Standard deviation = sqrt(66.67) ≈ 8.16 milliseconds
        assert!(results.std_dev.as_millis() >= 8);
        assert!(results.std_dev.as_millis() <= 9);
    }

    #[test]
    fn test_welford_vs_traditional_method() {
        // Test that Welford's method gives similar results to traditional method
        // but with better numerical stability
        let durations = vec![
            Duration::from_nanos(1000),
            Duration::from_nanos(2000),
            Duration::from_nanos(3000),
            Duration::from_nanos(4000),
            Duration::from_nanos(5000),
        ];
        let results = BenchmarkResults::from_durations(durations);

        // Mean should be 3000ns
        assert_eq!(results.mean, Duration::from_nanos(3000));

        // Standard deviation should be approximately sqrt(2000000) ≈ 1414ns
        assert!(results.std_dev.as_nanos() >= 1400);
        assert!(results.std_dev.as_nanos() <= 1500);
    }

    #[test]
    fn test_min_max_calculation() {
        let durations = vec![
            Duration::from_millis(10),
            Duration::from_millis(1),
            Duration::from_millis(5),
            Duration::from_millis(20),
            Duration::from_millis(3),
        ];
        let results = BenchmarkResults::from_durations(durations);
        assert_eq!(results.min, Duration::from_millis(1));
        assert_eq!(results.max, Duration::from_millis(20));
    }
}
