# Actor Testing Guide

> **Comprehensive testing strategies for zero-cost, platform-dual actors**

---

## 🎯 Introduction

Testing actor systems requires different strategies than traditional synchronous code. This guide covers:

- **Unit testing** individual actors with mock dependencies
- **Integration testing** actor supervision and lifecycle
- **Back-pressure testing** for both embedded and cloud platforms
- **Performance testing** with benchmarks and profiling
- **Property-based testing** for complex actor interactions

Our testing philosophy: **Test behavior, not implementation**. Focus on message flows, state transitions, and supervision contracts rather than internal actor mechanics.

---

## 🧪 Testing Fundamentals

### Test Structure Overview

```rust
// Standard test organization
#[cfg(test)]
mod tests {
    use super::*;
    use lit_bit_core::actor::test_utils::*;
    
    // Unit tests - single actor behavior
    mod unit {
        // Test message handling, state transitions
    }
    
    // Integration tests - actor interactions
    mod integration {
        // Test supervision, lifecycle, communication
    }
    
    // Performance tests - throughput and latency
    mod performance {
        // Benchmarks, load testing
    }
}
```

### Core Testing Utilities

```rust
use lit_bit_core::actor::test_utils::{
    TestKit,           // Actor testing framework
    MockActor,         // Configurable test actor
    MessageCapture,    // Record sent messages
    TimeController,    // Control time in tests
    SupervisionProbe,  // Monitor supervision events
};
```

---

## 🔬 Unit Testing Actors

### Basic Actor Testing

```rust
use lit_bit_core::actor::{Actor, ActorError};
use tokio::sync::oneshot;
use std::time::Duration;

#[derive(Debug)]
struct Calculator {
    value: i32,
}

#[derive(Debug, Clone)]
enum CalcMessage {
    Add(i32),
    Subtract(i32),
    Multiply(i32),
    GetValue { reply_to: oneshot::Sender<i32> },
    Reset,
}

impl Actor for Calculator {
    type Message = CalcMessage;
    
    async fn on_event(&mut self, msg: CalcMessage) {
        match msg {
            CalcMessage::Add(n) => self.value += n,
            CalcMessage::Subtract(n) => self.value -= n,
            CalcMessage::Multiply(n) => self.value *= n,
            CalcMessage::GetValue { reply_to } => {
                let _ = reply_to.send(self.value);
            },
            CalcMessage::Reset => self.value = 0,
        }
    }
}

#[cfg(test)]
mod calculator_tests {
    use super::*;
    use lit_bit_core::actor::test_utils::TestKit;
    
    #[tokio::test]
    async fn test_basic_arithmetic() {
        let mut testkit = TestKit::new();
        let calc = Calculator { value: 0 };
        let addr = testkit.spawn_actor::<Calculator, 16>(calc);
        
        // Test addition
        addr.send(CalcMessage::Add(5)).await.unwrap();
        addr.send(CalcMessage::Add(3)).await.unwrap();
        
        // Verify result
        let (tx, rx) = oneshot::channel();
        addr.send(CalcMessage::GetValue { reply_to: tx }).await.unwrap();
        let result = rx.await.unwrap();
        assert_eq!(result, 8);
        
        // Test multiplication
        addr.send(CalcMessage::Multiply(2)).await.unwrap();
        
        let (tx, rx) = oneshot::channel();
        addr.send(CalcMessage::GetValue { reply_to: tx }).await.unwrap();
        let result = rx.await.unwrap();
        assert_eq!(result, 16);
    }
    
    #[tokio::test]
    async fn test_reset_functionality() {
        let mut testkit = TestKit::new();
        let calc = Calculator { value: 42 };
        let addr = testkit.spawn_actor::<Calculator, 16>(calc);
        
        // Reset should clear value
        addr.send(CalcMessage::Reset).await.unwrap();
        
        let (tx, rx) = oneshot::channel();
        addr.send(CalcMessage::GetValue { reply_to: tx }).await.unwrap();
        let result = rx.await.unwrap();
        assert_eq!(result, 0);
    }
}
```

### Testing Actor Lifecycle

```rust
#[derive(Debug)]
struct ResourceActor {
    connection: Option<DatabaseConnection>,
    is_initialized: bool,
}

impl Actor for ResourceActor {
    type Message = ResourceMessage;
    
    fn on_start(&mut self) -> Result<(), ActorError> {
        // Simulate resource initialization
        self.connection = Some(DatabaseConnection::new()?);
        self.is_initialized = true;
        Ok(())
    }
    
    fn on_stop(self) -> Result<(), ActorError> {
        // Cleanup resources
        if let Some(conn) = self.connection {
            conn.close()?;
        }
        Ok(())
    }
    
    async fn on_event(&mut self, msg: ResourceMessage) {
        if !self.is_initialized {
            return; // Ignore messages before initialization
        }
        
        match msg {
            ResourceMessage::Query(sql) => {
                if let Some(ref conn) = self.connection {
                    let _ = conn.execute(sql).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use lit_bit_core::actor::test_utils::{TestKit, LifecycleProbe};
    
    #[tokio::test]
    async fn test_successful_initialization() {
        let mut testkit = TestKit::new();
        let mut probe = LifecycleProbe::new();
        
        let actor = ResourceActor {
            connection: None,
            is_initialized: false,
        };
        
        let addr = testkit.spawn_actor_with_probe::<ResourceActor, 16>(actor, &mut probe);
        
        // Wait for initialization
        probe.wait_for_start().await;
        
        // Verify actor is ready to receive messages
        addr.send(ResourceMessage::Query("SELECT 1".to_string())).await.unwrap();
        
        // Verify no errors during startup
        assert!(probe.start_errors().is_empty());
    }
    
    #[tokio::test]
    async fn test_initialization_failure() {
        let mut testkit = TestKit::new();
        let mut probe = LifecycleProbe::new();
        
        // Configure environment to cause initialization failure
        testkit.set_env("DB_URL", "invalid://connection");
        
        let actor = ResourceActor {
            connection: None,
            is_initialized: false,
        };
        
        let result = testkit.try_spawn_actor_with_probe::<ResourceActor, 16>(actor, &mut probe);
        
        // Should fail during on_start
        assert!(result.is_err());
        assert!(!probe.start_errors().is_empty());
    }
    
    #[tokio::test]
    async fn test_graceful_shutdown() {
        let mut testkit = TestKit::new();
        let mut probe = LifecycleProbe::new();
        
        let actor = ResourceActor {
            connection: None,
            is_initialized: false,
        };
        
        let addr = testkit.spawn_actor_with_probe::<ResourceActor, 16>(actor, &mut probe);
        
        // Send some messages
        addr.send(ResourceMessage::Query("SELECT 1".to_string())).await.unwrap();
        
        // Shutdown the actor
        testkit.shutdown_actor(addr).await;
        
        // Verify cleanup was called
        probe.wait_for_stop().await;
        assert!(probe.stop_errors().is_empty());
    }
}
```

---

## 🏗️ Integration Testing

### Testing Actor Communication

```rust
#[derive(Debug)]
struct Producer {
    consumer_addr: Option<Address<ConsumerMessage, 16>>,
    produced_count: u32,
}

#[derive(Debug)]
struct Consumer {
    processed_items: Vec<WorkItem>,
}

#[derive(Debug, Clone)]
enum ProducerMessage {
    SetConsumer(Address<ConsumerMessage, 16>),
    Produce(WorkItem),
}

#[derive(Debug, Clone)]
enum ConsumerMessage {
    Process(WorkItem),
    GetProcessedCount { reply_to: oneshot::Sender<usize> },
}

impl Actor for Producer {
    type Message = ProducerMessage;
    
    async fn on_event(&mut self, msg: ProducerMessage) {
        match msg {
            ProducerMessage::SetConsumer(addr) => {
                self.consumer_addr = Some(addr);
            },
            ProducerMessage::Produce(item) => {
                if let Some(ref addr) = self.consumer_addr {
                    let _ = addr.send(ConsumerMessage::Process(item)).await;
                    self.produced_count += 1;
                }
            }
        }
    }
}

impl Actor for Consumer {
    type Message = ConsumerMessage;
    
    async fn on_event(&mut self, msg: ConsumerMessage) {
        match msg {
            ConsumerMessage::Process(item) => {
                self.processed_items.push(item);
            },
            ConsumerMessage::GetProcessedCount { reply_to } => {
                let _ = reply_to.send(self.processed_items.len());
            }
        }
    }
}

#[cfg(test)]
mod communication_tests {
    use super::*;
    use lit_bit_core::actor::test_utils::TestKit;
    use tokio::sync::oneshot;
    
    #[tokio::test]
    async fn test_producer_consumer_flow() {
        let mut testkit = TestKit::new();
        
        // Spawn consumer first
        let consumer = Consumer { processed_items: Vec::new() };
        let consumer_addr = testkit.spawn_actor::<Consumer, 16>(consumer);
        
        // Spawn producer and connect to consumer
        let producer = Producer {
            consumer_addr: None,
            produced_count: 0,
        };
        let producer_addr = testkit.spawn_actor::<Producer, 16>(producer);
        
        // Connect producer to consumer
        producer_addr.send(ProducerMessage::SetConsumer(consumer_addr.clone())).await.unwrap();
        
        // Produce some items
        let items = vec![
            WorkItem::new("task1"),
            WorkItem::new("task2"),
            WorkItem::new("task3"),
        ];
        
        for item in items.clone() {
            producer_addr.send(ProducerMessage::Produce(item)).await.unwrap();
        }
        
        // Give time for processing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        // Verify all items were processed
        let (tx, rx) = oneshot::channel();
        consumer_addr.send(ConsumerMessage::GetProcessedCount { reply_to: tx }).await.unwrap();
        let processed_count = rx.await.unwrap();
        
        assert_eq!(processed_count, items.len());
    }
}
```

### Testing Supervision

```rust
#[derive(Debug)]
struct FlakyWorker {
    id: u32,
    failure_rate: f32, // 0.0 to 1.0
    processed_count: u32,
}

impl Actor for FlakyWorker {
    type Message = WorkerMessage;
    
    async fn on_event(&mut self, msg: WorkerMessage) {
        match msg {
            WorkerMessage::ProcessTask(task) => {
                // Simulate random failures
                if rand::random::<f32>() < self.failure_rate {
                    panic!("Worker {} failed on task: {:?}", self.id, task);
                }
                
                self.processed_count += 1;
            }
        }
    }
    
    fn on_panic(&self, _info: &std::panic::PanicInfo) -> RestartStrategy {
        RestartStrategy::OneForOne
    }
    
    fn on_start(&mut self) -> Result<(), ActorError> {
        // Reset state on restart
        self.processed_count = 0;
        Ok(())
    }
}

#[cfg(test)]
mod supervision_tests {
    use super::*;
    use lit_bit_core::actor::test_utils::{TestKit, SupervisionProbe};
    
    #[tokio::test]
    async fn test_actor_restart_on_panic() {
        let mut testkit = TestKit::new();
        let mut probe = SupervisionProbe::new();
        
        // Create a worker that always fails
        let worker = FlakyWorker {
            id: 1,
            failure_rate: 1.0, // Always fail
            processed_count: 0,
        };
        
        let addr = testkit.spawn_actor_with_supervision::<FlakyWorker, 16>(worker, &mut probe);
        
        // Send a task that will cause panic
        let task = WorkTask::new("test");
        addr.send(WorkerMessage::ProcessTask(task)).await.unwrap();
        
        // Wait for panic and restart
        probe.wait_for_panic().await;
        probe.wait_for_restart().await;
        
        // Verify restart occurred
        assert_eq!(probe.restart_count(), 1);
        assert_eq!(probe.last_restart_strategy(), Some(RestartStrategy::OneForOne));
    }
    
    #[tokio::test]
    async fn test_supervision_strategy_one_for_all() {
        let mut testkit = TestKit::new();
        let mut probe = SupervisionProbe::new();
        
        // Create supervisor with multiple workers
        let supervisor = WorkerSupervisor::new();
        let supervisor_addr = testkit.spawn_actor_with_supervision::<WorkerSupervisor, 32>(supervisor, &mut probe);
        
        // Spawn multiple workers under supervision
        for i in 0..3 {
            supervisor_addr.send(SupervisorMessage::SpawnWorker {
                id: i,
                failure_rate: if i == 1 { 1.0 } else { 0.0 }, // Only worker 1 fails
            }).await.unwrap();
        }
        
        // Configure OneForAll strategy
        supervisor_addr.send(SupervisorMessage::SetStrategy(RestartStrategy::OneForAll)).await.unwrap();
        
        // Send task to failing worker
        supervisor_addr.send(SupervisorMessage::DistributeWork {
            worker_id: 1,
            task: WorkTask::new("fail"),
        }).await.unwrap();
        
        // Wait for supervision event
        probe.wait_for_supervision_event().await;
        
        // Verify all workers were restarted
        assert_eq!(probe.restarted_actors().len(), 3);
        assert_eq!(probe.last_restart_strategy(), Some(RestartStrategy::OneForAll));
    }
}
```

---

## ⚡ Back-pressure Testing

### Embedded (no_std) Back-pressure

```rust
#[cfg(test)]
mod embedded_backpressure_tests {
    use super::*;
    use lit_bit_core::actor::test_utils::{EmbeddedTestKit, MessageCapture};
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_mailbox_overflow_handling() {
        let mut testkit = EmbeddedTestKit::new();
        let mut capture = MessageCapture::new();
        
        // Create actor with small mailbox
        let actor = SlowProcessor::new();
        let addr = testkit.spawn_actor::<SlowProcessor, 4>(actor); // Only 4 slots
        
        // Fill the mailbox
        let mut sent_messages = Vec::new();
        let mut failed_messages = Vec::new();
        
        for i in 0..10 {
            let msg = ProcessMessage::Work(i);
            match addr.try_send(msg.clone()) {
                Ok(()) => sent_messages.push(msg),
                Err(SendError::Full(msg)) => failed_messages.push(msg),
                Err(SendError::Closed(_)) => panic!("Actor died unexpectedly"),
            }
        }
        
        // Verify back-pressure behavior
        assert_eq!(sent_messages.len(), 4); // Mailbox capacity
        assert_eq!(failed_messages.len(), 6); // Overflow messages
        
        // Verify no messages were lost
        assert_eq!(sent_messages.len() + failed_messages.len(), 10);
    }
    
    #[tokio::test]
    async fn test_fail_fast_semantics() {
        let mut testkit = EmbeddedTestKit::new();
        
        let actor = SlowProcessor::new();
        let addr = testkit.spawn_actor::<SlowProcessor, 2>(actor);
        
        // Fill mailbox immediately
        addr.try_send(ProcessMessage::Work(1)).unwrap();
        addr.try_send(ProcessMessage::Work(2)).unwrap();
        
        // Next send should fail immediately (no blocking)
        let start = std::time::Instant::now();
        let result = addr.try_send(ProcessMessage::Work(3));
        let elapsed = start.elapsed();
        
        assert!(matches!(result, Err(SendError::Full(_))));
        assert!(elapsed < std::time::Duration::from_millis(1)); // Should be immediate
    }
    
    #[tokio::test]
    async fn test_message_ordering_under_pressure() {
        let mut testkit = EmbeddedTestKit::new();
        let mut capture = MessageCapture::new();
        
        let actor = OrderedProcessor::new();
        let addr = testkit.spawn_actor_with_capture::<OrderedProcessor, 8>(actor, &mut capture);
        
        // Send messages rapidly
        let messages: Vec<_> = (0..5).map(|i| ProcessMessage::Work(i)).collect();
        
        for msg in &messages {
            addr.try_send(msg.clone()).unwrap();
        }
        
        // Wait for processing
        testkit.advance_time(Duration::from_millis(100)).await;
        
        // Verify messages were processed in order
        let processed = capture.captured_messages();
        for (i, msg) in processed.iter().enumerate() {
            if let ProcessMessage::Work(id) = msg {
                assert_eq!(*id, i);
            }
        }
    }
}
```

### Cloud (std) Back-pressure

```rust
#[cfg(test)]
mod cloud_backpressure_tests {
    use super::*;
    use lit_bit_core::actor::test_utils::{CloudTestKit, LoadGenerator};
    use tokio::sync::oneshot;
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_async_backpressure() {
        let mut testkit = CloudTestKit::new();
        
        // Create slow processor with small mailbox
        let actor = SlowProcessor::new();
        let addr = testkit.spawn_actor::<SlowProcessor, 4>(actor);
        
        // Send messages faster than they can be processed
        let send_tasks: Vec<_> = (0..10)
            .map(|i| {
                let addr = addr.clone();
                tokio::spawn(async move {
                    let start = std::time::Instant::now();
                    let result = addr.send(ProcessMessage::Work(i)).await;
                    (i, start.elapsed(), result)
                })
            })
            .collect();
        
        // Wait for all sends to complete
        let results: Vec<_> = futures::future::join_all(send_tasks).await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();
        
        // Verify back-pressure caused delays
        let mut delayed_count = 0;
        for (id, elapsed, result) in results {
            assert!(result.is_ok(), "Message {} failed: {:?}", id, result);
            
            if elapsed > Duration::from_millis(10) {
                delayed_count += 1;
            }
        }
        
        // Some sends should have been delayed due to back-pressure
        assert!(delayed_count > 0, "Expected some sends to be delayed by back-pressure");
    }
    
    #[tokio::test]
    async fn test_load_shedding_pattern() {
        let mut testkit = CloudTestKit::new();
        let mut load_gen = LoadGenerator::new();
        
        // Create actor with load shedding capability
        let actor = LoadSheddingProcessor::new(Duration::from_millis(100)); // 100ms timeout
        let addr = testkit.spawn_actor::<LoadSheddingProcessor, 16>(actor);
        
        // Generate high load
        load_gen.configure()
            .rate(1000) // 1000 messages/sec
            .duration(Duration::from_secs(5))
            .message_factory(|| ProcessMessage::Work(rand::random()));
        
        let stats = load_gen.run_against(addr).await;
        
        // Verify load shedding occurred
        assert!(stats.sent_count > stats.processed_count);
        assert!(stats.timeout_count > 0);
        assert!(stats.average_latency < Duration::from_millis(200)); // Stayed responsive
    }
    
    #[tokio::test]
    async fn test_graceful_degradation() {
        let mut testkit = CloudTestKit::new();
        
        // Create circuit breaker actor
        let actor = CircuitBreakerActor::new();
        let addr = testkit.spawn_actor::<CircuitBreakerActor, 32>(actor);
        
        // Simulate downstream service failures
        for _ in 0..10 {
            let result = addr.send(ServiceMessage::Call("failing_service".to_string())).await;
            // Expect failures initially
        }
        
        // Circuit should be open now
        let (tx, rx) = oneshot::channel();
        addr.send(ServiceMessage::GetState { reply_to: tx }).await.unwrap();
        let state = rx.await.unwrap();
        
        assert_eq!(state, CircuitState::Open);
        
        // Subsequent calls should fail fast
        let start = std::time::Instant::now();
        let result = addr.send(ServiceMessage::Call("any_service".to_string())).await;
        let elapsed = start.elapsed();
        
        assert!(elapsed < Duration::from_millis(10)); // Should fail immediately
    }
}
```

---

## 📊 Performance Testing

### Throughput Benchmarks

```rust
#[cfg(test)]
mod performance_tests {
    use super::*;
    use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
    use lit_bit_core::actor::test_utils::{PerformanceTestKit, ThroughputMeter};
    use tokio::sync::oneshot;
    
    fn bench_message_throughput(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        let mut group = c.benchmark_group("message_throughput");
        
        for mailbox_size in [16, 64, 256, 1024].iter() {
            group.bench_with_input(
                BenchmarkId::new("tokio", mailbox_size),
                mailbox_size,
                |b, &size| {
                    b.to_async(&rt).iter(|| async {
                        let mut testkit = PerformanceTestKit::new();
                        let actor = ThroughputTestActor::new();
                        let addr = testkit.spawn_actor_tokio(actor, size);
                        
                        // Send 1000 messages and measure time
                        let start = std::time::Instant::now();
                        for i in 0..1000 {
                            addr.send(TestMessage::Increment(i)).await.unwrap();
                        }
                        
                        // Wait for processing to complete
                        let (tx, rx) = oneshot::channel();
                        addr.send(TestMessage::GetCount { reply_to: tx }).await.unwrap();
                        let count = rx.await.unwrap();
                        assert_eq!(count, 1000);
                        
                        start.elapsed()
                    });
                },
            );
        }
        
        group.finish();
    }
    
    fn bench_actor_spawn_cost(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        c.bench_function("actor_spawn_tokio", |b| {
            b.to_async(&rt).iter(|| async {
                let mut testkit = PerformanceTestKit::new();
                let actor = MinimalActor::new();
                
                let start = std::time::Instant::now();
                let _addr = testkit.spawn_actor_tokio::<MinimalActor, 16>(actor);
                start.elapsed()
            });
        });
        
        c.bench_function("actor_spawn_embassy", |b| {
            b.to_async(&rt).iter(|| async {
                let mut testkit = PerformanceTestKit::new();
                let actor = MinimalActor::new();
                
                let start = std::time::Instant::now();
                let _addr = testkit.spawn_actor_embassy::<MinimalActor, 16>(actor);
                start.elapsed()
            });
        });
    }
    
    criterion_group!(benches, bench_message_throughput, bench_actor_spawn_cost);
    criterion_main!(benches);
}
```

### Latency Measurements

```rust
#[cfg(test)]
mod latency_tests {
    use super::*;
    use lit_bit_core::actor::test_utils::{LatencyMeter, PerformanceTestKit};
    use tokio::sync::oneshot;
    use std::time::Duration;
    
    #[tokio::test]
    async fn measure_round_trip_latency() {
        let mut testkit = PerformanceTestKit::new();
        let mut latency_meter = LatencyMeter::new();
        
        let actor = EchoActor::new();
        let addr = testkit.spawn_actor::<EchoActor, 16>(actor);
        
        // Warm up
        for _ in 0..100 {
            let (tx, rx) = oneshot::channel();
            addr.send(EchoMessage::Echo { 
                data: "warmup".to_string(),
                reply_to: tx 
            }).await.unwrap();
            rx.await.unwrap();
        }
        
        // Measure latency over many iterations
        for i in 0..1000 {
            let (tx, rx) = oneshot::channel();
            
            let start = std::time::Instant::now();
            addr.send(EchoMessage::Echo { 
                data: format!("test_{}", i),
                reply_to: tx 
            }).await.unwrap();
            
            let response = rx.await.unwrap();
            let latency = start.elapsed();
            
            latency_meter.record(latency);
            assert_eq!(response, format!("echo: test_{}", i));
        }
        
        let stats = latency_meter.stats();
        println!("Latency stats:");
        println!("  Mean: {:?}", stats.mean);
        println!("  P50:  {:?}", stats.p50);
        println!("  P95:  {:?}", stats.p95);
        println!("  P99:  {:?}", stats.p99);
        println!("  Max:  {:?}", stats.max);
        
        // Assert performance targets
        assert!(stats.p95 < Duration::from_micros(500), "P95 latency too high: {:?}", stats.p95);
        assert!(stats.p99 < Duration::from_millis(1), "P99 latency too high: {:?}", stats.p99);
    }
    
    #[tokio::test]
    async fn measure_supervision_overhead() {
        let mut testkit = PerformanceTestKit::new();
        let mut latency_meter = LatencyMeter::new();
        
        // Test with supervision
        let supervised_actor = SupervisedActor::new();
        let supervised_addr = testkit.spawn_actor_with_supervision::<SupervisedActor, 16>(supervised_actor);
        
        // Test without supervision
        let unsupervised_actor = UnsupervisedActor::new();
        let unsupervised_addr = testkit.spawn_actor::<UnsupervisedActor, 16>(unsupervised_actor);
        
        // Measure supervised latency
        let supervised_latency = measure_actor_latency(&supervised_addr, 1000).await;
        
        // Measure unsupervised latency
        let unsupervised_latency = measure_actor_latency(&unsupervised_addr, 1000).await;
        
        let overhead = supervised_latency.mean - unsupervised_latency.mean;
        println!("Supervision overhead: {:?}", overhead);
        
        // Supervision overhead should be minimal
        assert!(overhead < Duration::from_nanos(100), "Supervision overhead too high: {:?}", overhead);
    }
}
```

---

## 🎲 Property-Based Testing

### Actor Invariants

```rust
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use lit_bit_core::actor::test_utils::{PropertyTestKit, MessageSequence};
    use tokio::sync::oneshot;
    use std::time::Duration;
    
    proptest! {
        #[test]
        fn actor_message_ordering_preserved(
            messages in prop::collection::vec(any::<u32>(), 1..100)
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mut testkit = PropertyTestKit::new();
                let actor = OrderPreservingActor::new();
                let addr = testkit.spawn_actor::<OrderPreservingActor, 128>(actor);
                
                // Send messages in order
                for (i, &value) in messages.iter().enumerate() {
                    addr.send(OrderedMessage::Process { 
                        sequence: i, 
                        value 
                    }).await.unwrap();
                }
                
                // Get processed results
                let (tx, rx) = oneshot::channel();
                addr.send(OrderedMessage::GetResults { reply_to: tx }).await.unwrap();
                let results = rx.await.unwrap();
                
                // Verify ordering preserved
                for (i, result) in results.iter().enumerate() {
                    prop_assert_eq!(result.sequence, i);
                    prop_assert_eq!(result.value, messages[i]);
                }
            });
        }
        
        #[test]
        fn supervision_always_recovers(
            failure_points in prop::collection::vec(0usize..100, 1..10)
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mut testkit = PropertyTestKit::new();
                let mut probe = SupervisionProbe::new();
                
                let actor = FlakyActor::new(failure_points.clone());
                let addr = testkit.spawn_actor_with_supervision::<FlakyActor, 32>(actor, &mut probe);
                
                // Send 100 messages
                for i in 0..100 {
                    addr.send(FlakyMessage::Process(i)).await.unwrap();
                }
                
                // Wait for all processing to complete
                testkit.wait_for_quiescence().await;
                
                // Verify system recovered from all failures
                let restart_count = probe.restart_count();
                prop_assert_eq!(restart_count, failure_points.len());
                
                // Verify final state is consistent
                let (tx, rx) = oneshot::channel();
                addr.send(FlakyMessage::GetProcessedCount { reply_to: tx }).await.unwrap();
                let processed = rx.await.unwrap();
                prop_assert_eq!(processed, 100 - failure_points.len()); // Failures don't count
            });
        }
        
        #[test]
        fn backpressure_never_loses_messages(
            send_pattern in prop::collection::vec(1u32..1000, 10..50)
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mut testkit = PropertyTestKit::new();
                let actor = CountingActor::new();
                let addr = testkit.spawn_actor::<CountingActor, 8>(actor); // Small mailbox
                
                let mut total_sent = 0;
                let mut total_failed = 0;
                
                for &batch_size in &send_pattern {
                    for i in 0..batch_size {
                        match addr.try_send(CountMessage::Increment) {
                            Ok(()) => total_sent += 1,
                            Err(SendError::Full(_)) => total_failed += 1,
                            Err(SendError::Closed(_)) => prop_assert!(false, "Actor died unexpectedly"),
                        }
                    }
                    
                    // Allow some processing between batches
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
                
                // Wait for all queued messages to process
                testkit.wait_for_quiescence().await;
                
                // Get final count
                let (tx, rx) = oneshot::channel();
                addr.send(CountMessage::GetCount { reply_to: tx }).await.unwrap();
                let final_count = rx.await.unwrap();
                
                // Verify no messages were lost (only failed sends should be missing)
                prop_assert_eq!(final_count, total_sent);
                prop_assert_eq!(total_sent + total_failed, send_pattern.iter().sum::<u32>());
            });
        }
    }
}
```

---

## 🛠️ Test Utilities Reference

### TestKit API

```rust
pub struct TestKit {
    // Core testing framework
}

impl TestKit {
    pub fn new() -> Self;
    
    // Actor spawning
    pub fn spawn_actor<A: Actor, const N: usize>(&mut self, actor: A) -> Address<A::Message, N>;
    pub fn spawn_actor_with_probe<A: Actor, const N: usize>(
        &mut self, 
        actor: A, 
        probe: &mut LifecycleProbe
    ) -> Address<A::Message, N>;
    
    // Time control
    pub async fn advance_time(&mut self, duration: Duration);
    pub fn pause_time(&mut self);
    pub fn resume_time(&mut self);
    
    // Environment control
    pub fn set_env(&mut self, key: &str, value: &str);
    pub fn clear_env(&mut self, key: &str);
    
    // Shutdown
    pub async fn shutdown_actor<M, const N: usize>(&mut self, addr: Address<M, N>);
    pub async fn shutdown_all(&mut self);
}
```

### Probes and Monitors

```rust
pub struct LifecycleProbe {
    // Monitor actor lifecycle events
}

impl LifecycleProbe {
    pub fn new() -> Self;
    pub async fn wait_for_start(&mut self);
    pub async fn wait_for_stop(&mut self);
    pub fn start_errors(&self) -> &[ActorError];
    pub fn stop_errors(&self) -> &[ActorError];
}

pub struct SupervisionProbe {
    // Monitor supervision events
}

impl SupervisionProbe {
    pub fn new() -> Self;
    pub async fn wait_for_panic(&mut self);
    pub async fn wait_for_restart(&mut self);
    pub fn restart_count(&self) -> usize;
    pub fn last_restart_strategy(&self) -> Option<RestartStrategy>;
    pub fn restarted_actors(&self) -> &[ActorId];
}

pub struct MessageCapture<M> {
    // Capture and inspect messages
}

impl<M> MessageCapture<M> {
    pub fn new() -> Self;
    pub fn captured_messages(&self) -> &[M];
    pub fn message_count(&self) -> usize;
    pub fn clear(&mut self);
}
```

### Performance Testing

```rust
pub struct PerformanceTestKit {
    // High-precision performance testing
}

impl PerformanceTestKit {
    pub fn new() -> Self;
    pub fn spawn_actor_tokio<A: Actor, const N: usize>(&mut self, actor: A) -> Address<A::Message, N>;
    pub fn spawn_actor_embassy<A: Actor, const N: usize>(&mut self, actor: A) -> Address<A::Message, N>;
    pub async fn wait_for_quiescence(&mut self);
}

pub struct LatencyMeter {
    // Collect latency statistics
}

impl LatencyMeter {
    pub fn new() -> Self;
    pub fn record(&mut self, latency: Duration);
    pub fn stats(&self) -> LatencyStats;
}

pub struct LatencyStats {
    pub mean: Duration,
    pub p50: Duration,
    pub p95: Duration,
    pub p99: Duration,
    pub max: Duration,
    pub min: Duration,
}
```

---

## 📋 Testing Checklist

### ✅ Unit Testing
- [ ] Test all message types and state transitions
- [ ] Test actor lifecycle (on_start, on_stop)
- [ ] Test error conditions and edge cases
- [ ] Test actor-specific business logic
- [ ] Verify resource cleanup

### ✅ Integration Testing
- [ ] Test actor-to-actor communication
- [ ] Test supervision strategies
- [ ] Test system-wide message flows
- [ ] Test graceful shutdown sequences
- [ ] Test error propagation

### ✅ Back-pressure Testing
- [ ] Test mailbox overflow (embedded)
- [ ] Test async back-pressure (cloud)
- [ ] Test message ordering under load
- [ ] Test load shedding patterns
- [ ] Test graceful degradation

### ✅ Performance Testing
- [ ] Benchmark message throughput
- [ ] Measure round-trip latency
- [ ] Test actor spawn costs
- [ ] Measure supervision overhead
- [ ] Profile memory usage

### ✅ Property Testing
- [ ] Test message ordering invariants
- [ ] Test supervision recovery properties
- [ ] Test back-pressure correctness
- [ ] Test system-wide invariants
- [ ] Test concurrent access patterns

---

## 🚀 Next Steps

### Advanced Testing Patterns
- **Chaos testing** - Random actor failures and network partitions
- **Fuzz testing** - Random message sequences and timing
- **Load testing** - Sustained high-throughput scenarios
- **Integration with external systems** - Database, network, file I/O

### Testing Infrastructure
- **CI/CD integration** - Automated testing in multiple environments
- **Performance regression detection** - Continuous benchmarking
- **Test result visualization** - Dashboards and reports
- **Test data management** - Fixtures, mocks, and test databases

### Documentation and Examples
- **Testing cookbook** - Common patterns and recipes
- **Troubleshooting guide** - Debug failing tests
- **Best practices** - Team testing standards
- **Example test suites** - Reference implementations

---

*Happy testing! 🧪 Remember: Good tests are the foundation of reliable actor systems.* 