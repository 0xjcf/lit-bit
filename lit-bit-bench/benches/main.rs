use criterion::{criterion_group, criterion_main};

mod actor_performance;
mod async_throughput;
mod transition_latency;

criterion_group!(
    name = benches;
    config = criterion::Criterion::default();
    targets =
        actor_performance::bench_actor_performance,
        async_throughput::bench_message_throughput,
        transition_latency::bench_state_transitions
);

criterion_main!(benches);
