//! Throughput benchmarks for statechart operations

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use lit_bit_bench::common::BenchEvent;

fn bench_statechart_transitions(c: &mut Criterion) {
    let mut group = c.benchmark_group("statechart_transitions");

    for num_transitions in &[100, 1000, 10000] {
        group.bench_with_input(
            BenchmarkId::new("sequential", num_transitions),
            num_transitions,
            |b, &num_transitions| {
                b.iter(|| {
                    // TODO: Implement when statechart is available
                    // For now, just simulate work
                    for i in 0..num_transitions {
                        black_box(i);
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_event_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("event_processing");

    let events: Vec<BenchEvent> = (0..1000).map(BenchEvent::Transition).collect();

    group.bench_function("batch_processing", |b| {
        b.iter(|| {
            // TODO: Implement batch event processing
            for event in &events {
                black_box(event);
            }
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
                    // TODO: Implement statechart creation benchmark
                    black_box(num_states);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_statechart_transitions,
    bench_event_processing,
    bench_state_machine_creation
);
criterion_main!(benches);
