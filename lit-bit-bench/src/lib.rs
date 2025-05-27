//! Performance benchmarks for lit-bit
//!
//! This crate contains Criterion and Iai-Callgrind benchmarks for measuring
//! the performance characteristics of lit-bit statecharts and actors.

pub mod fixtures;
pub mod utils;

/// Common benchmark utilities and test data
pub mod common {

    /// Generate a large statechart for performance testing
    /// TODO: Implement when statechart macro is available
    pub fn create_large_statechart() {
        // Placeholder for statechart creation
        unimplemented!("Statechart creation for benchmarks")
    }

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
}
