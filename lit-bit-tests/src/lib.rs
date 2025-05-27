//! Integration and property tests for lit-bit
//!
//! This crate contains comprehensive tests that require std features
//! and heavy dependencies that shouldn't be part of the core `no_std` build.

#![cfg(test)]

pub mod actor_tests;
pub mod async_tests;
pub mod integration;
pub mod property_tests;

/// Common test utilities and fixtures
pub mod common {

    /// Setup tracing for tests
    pub fn setup_tracing() {
        use tracing_subscriber::{EnvFilter, fmt};

        let _ = fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .try_init();
    }

    /// Common test events for statechart testing
    #[derive(Debug, Clone, PartialEq)]
    pub enum TestEvent {
        Start,
        Stop,
        Reset,
        Tick,
    }

    /// Common test states for statechart testing
    #[derive(Debug, Clone, PartialEq)]
    pub enum TestState {
        Idle,
        Running,
        Stopped,
        Error,
    }
}
