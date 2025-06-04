use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use lit_bit_core::statechart;
use lit_bit_macro::statechart_event;

// Simple state machine for benchmarking
#[derive(Debug, Clone, Default)]
pub struct BenchContext {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[statechart_event]
pub enum BenchEvent {
    Toggle,
}

statechart! {
    name: BenchStateMachine,
    context: BenchContext,
    event: BenchEvent,
    initial: StateA,

    state StateA {
        on BenchEvent::Toggle => StateB;
    }

    state StateB {
        on BenchEvent::Toggle => StateA;
    }
}

pub fn bench_state_transitions(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_transitions");
    group.throughput(Throughput::Elements(1));

    // Pure transition overhead - machine created once outside the loop
    group.bench_function("pure_transition", |b| {
        let mut machine = BenchStateMachine::new(BenchContext::default(), &BenchEvent::Toggle)
            .expect("Failed to create state machine");
        b.iter(|| machine.send(&BenchEvent::Toggle));
    });

    // Transition with initialization overhead - machine created inside the loop
    group.bench_function("transition_with_init", |b| {
        b.iter(|| {
            let mut machine = BenchStateMachine::new(BenchContext::default(), &BenchEvent::Toggle)
                .expect("Failed to create state machine");
            machine.send(&BenchEvent::Toggle)
        });
    });

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default();
    targets = bench_state_transitions
);

criterion_main!(benches);
