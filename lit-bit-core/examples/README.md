# Actor System Examples

This directory contains comprehensive examples demonstrating the lit-bit actor system's capabilities. Each example showcases different aspects of the zero-cost, platform-dual actor framework.

## 🎯 Overview

The lit-bit actor system brings **mature actor model patterns** to Rust statecharts with unique features:

- **Zero-cost StateMachine integration** - Your statecharts become actors with no runtime overhead
- **Platform-dual design** - Same code runs on embedded (`no_std` + Embassy) and cloud (`std` + Tokio)
- **Supervision-aware lifecycle** - OTP-inspired restart strategies prevent cascading failures
- **Research-validated performance** - <200ns message latency, >100k messages/sec/core targets

## 📚 Example Categories

### 1. Core Statechart Examples

These examples demonstrate the foundational statechart functionality:

#### `traffic_light.rs`
- **Focus**: Basic statechart with RISC-V no_std target
- **Concepts**: State transitions, action functions, embedded deployment
- **Platform**: RISC-V embedded (no_std)

#### `traffic_light_cortex_m.rs`
- **Focus**: Cortex-M ARM embedded deployment
- **Concepts**: Cross-platform embedded support, memory constraints
- **Platform**: ARM Cortex-M (no_std)

#### `media_player.rs`
- **Focus**: Parallel states and complex state hierarchies
- **Concepts**: Concurrent regions, orthogonal states, complex event handling
- **Platform**: Both std and no_std

#### `external_events.rs`
- **Focus**: External event integration and async patterns
- **Concepts**: Event sourcing, external triggers, async coordination
- **Platform**: Primarily std

### 2. Actor System Examples

These examples demonstrate the actor layer built on top of statecharts:

#### `actor_statechart_integration.rs` ⭐
- **Focus**: Zero-cost StateMachine → Actor conversion
- **Concepts**: 
  - Automatic Actor implementation for StateMachine types
  - Zero-cost abstraction - no boxing or dynamic dispatch
  - Type-safe message passing with statechart events
  - Supervision-aware lifecycle hooks
  - Platform-dual design
- **Key Value**: Shows how existing statecharts become actors automatically

#### `actor_calculator.rs`
- **Focus**: Basic actor implementation and message passing
- **Concepts**:
  - Actor trait implementation
  - Request-response patterns with oneshot channels
  - Actor lifecycle management (on_start, on_stop)
  - Error handling and state encapsulation
- **Key Value**: Foundation patterns for actor development

#### `actor_backpressure.rs`
- **Focus**: Platform-specific back-pressure handling
- **Concepts**:
  - Embedded (no_std): Fail-fast semantics with immediate feedback
  - Cloud (std): Async back-pressure with natural flow control
  - Mailbox overflow handling
  - Load shedding patterns
- **Key Value**: Shows how the same actor code adapts to different platforms

### 3. Utility Examples

#### `heap_crash.rs`
- **Focus**: Demonstrating heapless behavior in no_std environments
- **Concepts**: Memory safety, heap allocation detection, embedded constraints
- **Platform**: no_std with heap detection

## 🚀 Running Examples

### Prerequisites

```bash
# Install Rust with the required targets
rustup target add riscv32imac-unknown-none-elf
rustup target add thumbv7em-none-eabihf

# For RISC-V examples, install QEMU
# macOS:
brew install qemu
# Ubuntu:
sudo apt-get install qemu-system-riscv32
```

### Standard (std) Examples

```bash
# Run with std features (Tokio-based)
cargo run --example actor_statechart_integration --features std
cargo run --example actor_calculator --features std
cargo run --example actor_backpressure --features std
cargo run --example media_player --features std
```

### Embedded (no_std) Examples

```bash
# RISC-V target
cargo run --example traffic_light --target riscv32imac-unknown-none-elf --release

# ARM Cortex-M target  
cargo run --example traffic_light_cortex_m --target thumbv7em-none-eabihf --release

# Test no_std compilation
cargo check --example actor_statechart_integration --target riscv32imac-unknown-none-elf --no-default-features
```

### Embassy Examples (when available)

```bash
# With Embassy async runtime
cargo run --example actor_backpressure --target thumbv7em-none-eabihf --features embassy --release
```

## 🧪 Testing Examples

```bash
# Run all example tests
cargo test --examples --features std

# Test specific example
cargo test --example actor_calculator --features std

# Test no_std compilation
cargo check --examples --target riscv32imac-unknown-none-elf --no-default-features
```

## 📖 Learning Path

### 1. Start with Statecharts
Begin with the statechart examples to understand the foundation:
1. `traffic_light.rs` - Basic state machine concepts
2. `media_player.rs` - Parallel states and complex hierarchies

### 2. Understand Actor Integration
Move to actor examples to see the value proposition:
1. `actor_statechart_integration.rs` - See how statecharts become actors
2. `actor_calculator.rs` - Learn basic actor patterns

### 3. Explore Platform Differences
Understand how the same code adapts to different environments:
1. `actor_backpressure.rs` - Platform-specific behavior
2. Compare std vs no_std compilation of the same examples

### 4. Advanced Patterns
Explore more complex scenarios:
1. `external_events.rs` - Integration with external systems
2. Custom supervision strategies
3. Performance optimization techniques

## 🔧 Development Tips

### Adding New Examples

1. **Follow naming convention**: `category_specific_name.rs`
2. **Include comprehensive documentation**: Explain concepts and use cases
3. **Support both platforms**: Use `#[cfg(feature = "std")]` appropriately
4. **Add tests**: Include unit tests demonstrating key functionality
5. **Update this README**: Add your example to the appropriate category

### Platform-Specific Code

```rust
// Use conditional compilation for platform differences
#[cfg(feature = "std")]
use std::collections::HashMap;

#[cfg(not(feature = "std"))]
use heapless::FnvIndexMap as HashMap;

// Platform-specific implementations
#[cfg(feature = "std")]
async fn tokio_specific_function() {
    tokio::time::sleep(Duration::from_millis(100)).await;
}

#[cfg(not(feature = "std"))]
async fn embassy_specific_function() {
    embassy_time::Timer::after(embassy_time::Duration::from_millis(100)).await;
}
```

### Memory Considerations

For no_std examples:
- Use `heapless` collections with explicit capacity
- Avoid `String` - use `heapless::String<N>` instead
- Be mindful of stack usage
- Use `const` generics for compile-time sizing

## 🎯 Key Concepts Demonstrated

### Zero-Cost Abstractions
- StateMachine → Actor conversion with no runtime overhead
- Compile-time type safety without dynamic dispatch
- Platform-specific optimizations

### Platform-Dual Design
- Same actor code runs on embedded and cloud
- Automatic adaptation to platform capabilities
- Consistent API across environments

### Supervision Patterns
- OTP-inspired restart strategies
- Lifecycle management hooks
- Error isolation and recovery

### Performance Characteristics
- <200ns message latency targets
- >100k messages/sec/core throughput
- Minimal memory footprint on embedded

## 🔗 Related Documentation

- [Actor Overview](../docs/actor-overview.md) - Comprehensive actor system guide
- [Test Guide](../docs/test-guide.md) - Testing strategies and utilities
- [ROADMAP](../ROADMAP.md) - Development roadmap and milestones

## 🤝 Contributing

When adding new examples:

1. **Focus on teaching**: Each example should clearly demonstrate specific concepts
2. **Real-world relevance**: Show patterns that developers will actually use
3. **Platform awareness**: Consider both embedded and cloud use cases
4. **Performance consciousness**: Demonstrate zero-cost principles
5. **Comprehensive testing**: Include tests that validate the concepts

## 📊 Performance Benchmarks

Some examples include performance measurements:

```bash
# Run benchmarks (when available)
cargo bench --example actor_calculator --features std

# Profile memory usage
cargo run --example actor_backpressure --features std -- --profile-memory
```

---

*Happy coding! 🦀 These examples showcase the power of zero-cost, platform-dual actors in Rust.* 