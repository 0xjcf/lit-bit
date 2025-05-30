//! Throughput benchmarks for statechart operations

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use lit_bit_core::SendResult;

// Import working statechart from tests
use std::convert::TryFrom;

const ACTION_LOG_CAPACITY: usize = 20;
const ACTION_STRING_CAPACITY: usize = 32;

#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkContext {
    count: i32,
    action_log: heapless::Vec<heapless::String<ACTION_STRING_CAPACITY>, ACTION_LOG_CAPACITY>,
}

impl Default for BenchmarkContext {
    fn default() -> Self {
        BenchmarkContext {
            count: 0,
            action_log: heapless::Vec::new(),
        }
    }
}

impl BenchmarkContext {
    fn record(&mut self, action_name: &str) {
        let s = heapless::String::try_from(action_name).unwrap_or_else(|_| heapless::String::new());
        let _ = self.action_log.push(s);
    }

    fn increment(&mut self) {
        self.count += 1;
    }

    fn reset(&mut self) {
        self.count = 0;
        self.action_log.clear();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BenchmarkEvent {
    #[default]
    Increment,
    Decrement,
    Reset,
    Start,
    Stop,
}

// Action functions for benchmarks
fn entry_state(context: &mut BenchmarkContext, _event: &BenchmarkEvent) {
    context.record("entry");
}

fn exit_state(context: &mut BenchmarkContext, _event: &BenchmarkEvent) {
    context.record("exit");
}

fn increment_action(context: &mut BenchmarkContext, _event: &BenchmarkEvent) {
    context.increment();
    context.record("increment");
}

fn reset_action(context: &mut BenchmarkContext, _event: &BenchmarkEvent) {
    context.reset();
    context.record("reset");
}

fn guard_can_increment(context: &BenchmarkContext, _event: &BenchmarkEvent) -> bool {
    context.count < 1000
}

// Import the statechart macro
use lit_bit_macro::statechart;

// Define a benchmark statechart
statechart! {
    name: BenchmarkMachine,
    context: BenchmarkContext,
    event: BenchmarkEvent,
    initial: Idle,

    state Idle {
        entry: entry_state;
        exit: exit_state;
        on BenchmarkEvent::Start => Active [action increment_action];
        on BenchmarkEvent::Reset => Idle [action reset_action];
    }

    state Active {
        entry: entry_state;
        exit: exit_state;
        on BenchmarkEvent::Increment [guard guard_can_increment] => Active [action increment_action];
        on BenchmarkEvent::Stop => Idle;
        on BenchmarkEvent::Reset => Idle [action reset_action];
    }
}

fn bench_statechart_transitions(c: &mut Criterion) {
    let mut group = c.benchmark_group("statechart_transitions");

    for num_transitions in &[100, 1000, 10000] {
        // Pre-allocate events vector outside the benchmark to avoid allocation overhead
        let events: Vec<BenchmarkEvent> = (0..*num_transitions)
            .map(|i| {
                if i % 100 == 0 {
                    BenchmarkEvent::Reset
                } else {
                    BenchmarkEvent::Increment
                }
            })
            .collect();

        group.throughput(Throughput::Elements(*num_transitions as u64));
        group.bench_with_input(
            BenchmarkId::new("sequential", num_transitions),
            num_transitions,
            |b, &_num_transitions| {
                b.iter(|| {
                    let mut machine = BenchmarkMachine::new(
                        BenchmarkContext::default(),
                        &BenchmarkEvent::default(),
                    )
                    .expect("Failed to create benchmark machine");

                    // Start the machine
                    let result = machine.send(&BenchmarkEvent::Start);
                    assert_eq!(result, SendResult::Transitioned);

                    // Process pre-allocated events to measure only statechart processing time
                    for event in &events {
                        let result = machine.send(event);
                        black_box(result);
                    }

                    black_box(machine.context().count);
                });
            },
        );
    }

    group.finish();
}

fn bench_event_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("event_processing");

    let events: Vec<BenchmarkEvent> = (0..1000)
        .map(|i| match i % 4 {
            0 => BenchmarkEvent::Increment,
            1 => BenchmarkEvent::Start,
            2 => BenchmarkEvent::Stop,
            _ => BenchmarkEvent::Reset,
        })
        .collect();

    group.throughput(Throughput::Elements(events.len() as u64));
    group.bench_function("batch_processing", |b| {
        b.iter(|| {
            let mut machine =
                BenchmarkMachine::new(BenchmarkContext::default(), &BenchmarkEvent::default())
                    .expect("Failed to create benchmark machine");

            // Process all events in batch
            for event in &events {
                let result = machine.send(event);
                black_box(result);
            }

            black_box(machine.context().count);
        });
    });

    group.finish();
}

fn bench_state_machine_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_machine_creation");

    for num_states in &[10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("creation", num_states),
            num_states,
            |b, &num_states| {
                b.iter(|| {
                    // Create multiple state machines to simulate scaling
                    let machines: Vec<_> = (0..num_states)
                        .map(|_| {
                            BenchmarkMachine::new(
                                BenchmarkContext::default(),
                                &BenchmarkEvent::default(),
                            )
                            .expect("Failed to create benchmark machine")
                        })
                        .collect();

                    black_box(machines);
                });
            },
        );
    }

    group.finish();
}

fn bench_guard_evaluation(c: &mut Criterion) {
    let mut group = c.benchmark_group("guard_evaluation");

    group.bench_function("guard_success_rate", |b| {
        b.iter(|| {
            let mut machine =
                BenchmarkMachine::new(BenchmarkContext::default(), &BenchmarkEvent::default())
                    .expect("Failed to create benchmark machine");

            // Start the machine
            machine.send(&BenchmarkEvent::Start);

            // Test guard evaluation - should succeed for first 1000 increments
            for _ in 0..500 {
                let result = machine.send(&BenchmarkEvent::Increment);
                assert_eq!(result, SendResult::Transitioned);
            }

            black_box(machine.context().count);
        });
    });

    group.finish();
}

fn bench_throughput_target_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_targets");

    // Pre-allocate events vector outside the benchmark to avoid allocation overhead
    let events: Vec<BenchmarkEvent> = (0..1_000_000)
        .map(|i| {
            if i % 1000 == 0 {
                BenchmarkEvent::Reset
            } else {
                BenchmarkEvent::Increment
            }
        })
        .collect();

    // Test to validate we meet the 1M events/sec target
    group.throughput(Throughput::Elements(events.len() as u64));
    group.bench_function("million_events_per_second", |b| {
        b.iter(|| {
            let mut machine =
                BenchmarkMachine::new(BenchmarkContext::default(), &BenchmarkEvent::default())
                    .expect("Failed to create benchmark machine");

            machine.send(&BenchmarkEvent::Start);

            // Process pre-allocated events to measure only statechart processing time
            for event in &events {
                let result = machine.send(event);
                black_box(result);
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_statechart_transitions,
    bench_event_processing,
    bench_state_machine_creation,
    bench_guard_evaluation,
    bench_throughput_target_validation
);
criterion_main!(benches);
