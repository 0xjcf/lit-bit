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
/// Panics if any arithmetic operation overflows or if the result is larger than `u32::MAX`.
#[must_use]
pub fn memory_intensive_scenario(num_batches: usize, batch_size: usize) -> Vec<BenchEvent> {
    (0..num_batches)
        .map(|i| {
            let batch: Vec<u32> = (0..batch_size)
                .map(|j| {
                    // Use checked arithmetic to prevent silent overflow
                    let base = i
                        .checked_mul(batch_size)
                        .expect("Overflow in i * batch_size calculation");
                    let index = base
                        .checked_add(j)
                        .expect("Overflow in base + j calculation");
                    u32::try_from(index).expect("Index too large to fit in u32")
                })
                .collect();
            BenchEvent::Batch(batch)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_intensive_scenario_normal_case() {
        let events = memory_intensive_scenario(2, 3);
        assert_eq!(events.len(), 2);

        // First batch should contain [0, 1, 2]
        if let BenchEvent::Batch(batch1) = &events[0] {
            assert_eq!(batch1, &[0, 1, 2]);
        } else {
            panic!("Expected first event to be a batch");
        }

        // Second batch should contain [3, 4, 5]
        if let BenchEvent::Batch(batch2) = &events[1] {
            assert_eq!(batch2, &[3, 4, 5]);
        } else {
            panic!("Expected second event to be a batch");
        }
    }

    #[test]
    fn test_memory_intensive_scenario_single_batch() {
        let events = memory_intensive_scenario(1, 5);
        assert_eq!(events.len(), 1);

        if let BenchEvent::Batch(batch) = &events[0] {
            assert_eq!(batch, &[0, 1, 2, 3, 4]);
        } else {
            panic!("Expected event to be a batch");
        }
    }

    #[test]
    fn test_memory_intensive_scenario_empty_batch() {
        let events = memory_intensive_scenario(2, 0);
        assert_eq!(events.len(), 2);

        for event in &events {
            if let BenchEvent::Batch(batch) = event {
                assert!(batch.is_empty());
            } else {
                panic!("Expected event to be a batch");
            }
        }
    }

    #[test]
    fn test_memory_intensive_scenario_validates_inputs() {
        // Test that the function works correctly with valid inputs
        // and that our overflow protection is in place (even if we can't easily trigger it)

        // Test with small reasonable values
        let events = memory_intensive_scenario(2, 5);
        assert_eq!(events.len(), 2);

        // Verify the arithmetic is correct
        if let BenchEvent::Batch(batch) = &events[1] {
            // Second batch should start at index 5 (1 * 5 + 0)
            assert_eq!(batch[0], 5);
            assert_eq!(batch[4], 9); // Last element should be 5 + 4
        } else {
            panic!("Expected second event to be a batch");
        }
    }

    #[test]
    fn test_overflow_protection_with_reasonable_values() {
        // Test that our overflow protection works with small, reasonable values
        let result = std::panic::catch_unwind(|| {
            // This should work fine - small reasonable values
            memory_intensive_scenario(10, 10)
        });
        assert!(
            result.is_ok(),
            "Should not panic with small reasonable values"
        );

        // Test with slightly larger but still reasonable values
        let result = std::panic::catch_unwind(|| {
            // This should work - still reasonable
            memory_intensive_scenario(100, 100)
        });
        assert!(result.is_ok(), "Should not panic with moderate values");
    }

    #[test]
    fn test_generate_event_sequence() {
        let events = generate_event_sequence(6);
        assert_eq!(events.len(), 6);

        // Check the pattern: Transition, Batch, Reset, Transition, Batch, Reset
        assert!(matches!(events[0], BenchEvent::Transition(0)));
        assert!(matches!(events[1], BenchEvent::Batch(_)));
        assert!(matches!(events[2], BenchEvent::Reset));
        assert!(matches!(events[3], BenchEvent::Transition(3)));
        assert!(matches!(events[4], BenchEvent::Batch(_)));
        assert!(matches!(events[5], BenchEvent::Reset));
    }

    #[test]
    fn test_create_large_batch() {
        let event = create_large_batch(5);
        if let BenchEvent::Batch(batch) = event {
            assert_eq!(batch, vec![0, 1, 2, 3, 4]);
        } else {
            panic!("Expected a batch event");
        }
    }

    #[test]
    fn test_realistic_workload() {
        let events = realistic_workload(10);
        assert_eq!(events.len(), 10);

        // Check that we have the expected distribution
        let transitions = events
            .iter()
            .filter(|e| matches!(e, BenchEvent::Transition(_)))
            .count();
        let batches = events
            .iter()
            .filter(|e| matches!(e, BenchEvent::Batch(_)))
            .count();
        let resets = events
            .iter()
            .filter(|e| matches!(e, BenchEvent::Reset))
            .count();

        assert_eq!(transitions, 7); // 70% of 10
        assert_eq!(batches, 2); // 20% of 10
        assert_eq!(resets, 1); // 10% of 10
    }

    #[test]
    fn test_stress_test_scenario() {
        let events = stress_test_scenario(5);
        assert_eq!(events.len(), 5);

        for (i, event) in events.iter().enumerate() {
            if let BenchEvent::Transition(id) = event {
                assert_eq!(*id, u32::try_from(i).unwrap());
            } else {
                panic!("Expected all events to be transitions");
            }
        }
    }
}
