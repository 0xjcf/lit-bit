//! Performance benchmarks for lit-bit
//!
//! This crate contains Criterion and Iai-Callgrind benchmarks for measuring
//! the performance characteristics of lit-bit statecharts and actors.

pub mod fixtures;
pub mod utils;

/// Common benchmark utilities and test data
pub mod common {
    /// Common events for benchmark testing
    #[derive(Debug, Clone, PartialEq)]
    pub enum BenchEvent {
        Transition(u32),
        Batch(Vec<u32>),
        Reset,
    }

    /// Performance test configuration
    pub struct BenchConfig {
        pub num_states: usize,
        pub num_events: usize,
        pub batch_size: usize,
    }

    impl Default for BenchConfig {
        fn default() -> Self {
            Self {
                num_states: 100,
                num_events: 1000,
                batch_size: 10,
            }
        }
    }

    /// Benchmark result analysis utilities
    pub struct BenchmarkReport {
        pub throughput_events_per_sec: f64,
        pub latency_nanos: f64,
        pub memory_bytes_per_operation: usize,
        pub meets_kpi_targets: bool,
    }

    impl BenchmarkReport {
        /// Check if performance meets the project KPI targets
        pub fn validate_against_kpis(&self) -> ValidationReport {
            let throughput_ok = self.throughput_events_per_sec >= 1_000_000.0; // 1M events/s target
            let latency_ok = self.latency_nanos <= 100.0; // <100ns latency target  
            let memory_ok = self.memory_bytes_per_operation <= 512; // ≤512B memory target

            ValidationReport {
                throughput_target_met: throughput_ok,
                latency_target_met: latency_ok,
                memory_target_met: memory_ok,
                overall_pass: throughput_ok && latency_ok && memory_ok,
            }
        }
    }

    pub struct ValidationReport {
        pub throughput_target_met: bool,
        pub latency_target_met: bool,
        pub memory_target_met: bool,
        pub overall_pass: bool,
    }

    /// Generate performance summary for Sprint 3 validation
    pub fn generate_performance_summary(reports: Vec<BenchmarkReport>) -> String {
        let mut summary = String::new();
        summary.push_str("🎯 Sprint 3 Performance Validation Summary\n");
        summary.push_str("==========================================\n\n");

        let mut all_pass = true;
        for (i, report) in reports.iter().enumerate() {
            let validation = report.validate_against_kpis();
            summary.push_str(&format!(
                "Benchmark {}: {}\n",
                i + 1,
                if validation.overall_pass {
                    "✅ PASS"
                } else {
                    "❌ FAIL"
                }
            ));

            if !validation.overall_pass {
                all_pass = false;
            }
        }

        summary.push_str(&format!(
            "\n🏆 Overall Sprint 3 Status: {}\n",
            if all_pass {
                "✅ ALL KPIs MET"
            } else {
                "❌ NEEDS IMPROVEMENT"
            }
        ));

        summary
    }
}
