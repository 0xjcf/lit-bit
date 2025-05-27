//! Benchmark fixtures for generating test data and scenarios

use crate::common::BenchEvent;

/// Generate a sequence of benchmark events
///
/// # Panics
///
/// Panics if `count` is larger than `u32::MAX`.
#[must_use]
pub fn generate_event_sequence(count: usize) -> Vec<BenchEvent> {
    (0..count)
        .map(|i| match i % 3 {
            0 => BenchEvent::Transition(u32::try_from(i).unwrap()),
            1 => BenchEvent::Batch(vec![
                u32::try_from(i).unwrap(),
                u32::try_from(i + 1).unwrap(),
            ]),
            _ => BenchEvent::Reset,
        })
        .collect()
}

/// Create a large batch of events for stress testing
///
/// # Panics
///
/// Panics if `size` is larger than `u32::MAX`.
#[must_use]
pub fn create_large_batch(size: usize) -> BenchEvent {
    let events: Vec<u32> = (0..size).map(|i| u32::try_from(i).unwrap()).collect();
    BenchEvent::Batch(events)
}

/// Generate a realistic workload pattern
///
/// # Panics
///
/// Panics if `duration_events` is larger than `u32::MAX`.
#[must_use]
pub fn realistic_workload(duration_events: usize) -> Vec<BenchEvent> {
    let mut events = Vec::with_capacity(duration_events);

    // Simulate a realistic pattern:
    // - 70% single transitions
    // - 20% small batches
    // - 10% resets
    for i in 0..duration_events {
        let event = match i % 10 {
            0..=6 => BenchEvent::Transition(u32::try_from(i).unwrap()),
            7..=8 => BenchEvent::Batch(vec![
                u32::try_from(i).unwrap(),
                u32::try_from(i + 1).unwrap(),
            ]),
            _ => BenchEvent::Reset,
        };
        events.push(event);
    }

    events
}

/// Create a stress test scenario with many rapid transitions
///
/// # Panics
///
/// Panics if `num_transitions` is larger than `u32::MAX`.
#[must_use]
pub fn stress_test_scenario(num_transitions: usize) -> Vec<BenchEvent> {
    (0..num_transitions)
        .map(|i| BenchEvent::Transition(u32::try_from(i).unwrap()))
        .collect()
}

/// Generate a memory-intensive scenario with large batches
///
/// # Panics
///
/// Panics if `num_batches * batch_size` is larger than `u32::MAX`.
#[must_use]
pub fn memory_intensive_scenario(num_batches: usize, batch_size: usize) -> Vec<BenchEvent> {
    (0..num_batches)
        .map(|i| {
            let batch: Vec<u32> = (0..batch_size)
                .map(|j| u32::try_from(i * batch_size + j).unwrap())
                .collect();
            BenchEvent::Batch(batch)
        })
        .collect()
}
