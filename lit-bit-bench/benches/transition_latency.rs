use criterion::{Criterion, Throughput, criterion_group};
use lit_bit_core::statechart;
use lit_bit_macro::statechart_event;
use tokio::runtime::Builder as TokioBuilder;

// Simple state machine for benchmarking
#[derive(Debug, Clone, Default)]
pub struct BenchContext {
    #[allow(dead_code)] // Used by statechart macro
    counter: u32,
}

impl BenchContext {
    pub fn new() -> Self {
        Self::default()
    }
}

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

    // Sync baseline
    group.bench_function("sync_transition", |b| {
        let mut machine = BenchStateMachine::new(BenchContext::new(), &BenchEvent::Toggle)
            .expect("Failed to create state machine");
        b.iter(|| machine.send(&BenchEvent::Toggle));
    });

    // Async comparison with Tokio
    let rt = TokioBuilder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    group.bench_function("async_transition", |b| {
        let mut machine = BenchStateMachine::new(BenchContext::new(), &BenchEvent::Toggle)
            .expect("Failed to create state machine");
        b.iter(|| rt.block_on(async { machine.send(&BenchEvent::Toggle) }));
    });

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default();
    targets = bench_state_transitions
);
