//! Test utilities for async actors across Tokio and Embassy runtimes
//!
//! This module provides cross-runtime test infrastructure with deterministic scheduling
//! and zero-overhead probes for testing async actor systems. Only available with
//! `test` or `test-probes` feature to ensure zero cost in production builds.

#[cfg(any(test, feature = "test-probes"))]
pub mod instrumented_actor;
#[cfg(any(test, feature = "test-probes"))]
pub mod probes;
#[cfg(any(test, feature = "test-probes"))]
pub mod test_kit;

// Re-exports for convenient usage
#[cfg(any(test, feature = "test-probes"))]
pub use instrumented_actor::InstrumentedActor;
#[cfg(any(test, feature = "test-probes"))]
pub use probes::{ActorProbe, ProbeEvent, TestError};
#[cfg(any(test, feature = "test-probes"))]
pub use test_kit::TestKit;
