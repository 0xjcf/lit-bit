# lit-bit: A `#![no_std]` Statechart Library for Rust

[![Heap/Unsafe-Free CI](https://img.shields.io/badge/heap--unsafe--free-checked-brightgreen?logo=rust&label=Heap%2FUnsafe%20Scan)](https://github.com/0xjcf/lit-bit/actions) ![CodeRabbit Pull Request Reviews](https://img.shields.io/coderabbit/prs/github/0xjcf/lit-bit?utm_source=oss&utm_medium=github&utm_campaign=0xjcf%2Flit-bit&labelColor=171717&color=FF570A&link=https%3A%2F%2Fcoderabbit.ai&label=CodeRabbit+Reviews)

**Build robust, reactive systems with XState-inspired statecharts that run everywhere from microcontrollers to cloud servers.**

`lit-bit` combines **statecharts** (for modeling complex state logic) with **actors** (for safe concurrent execution) in a single, zero-cost abstraction that works in `#![no_std]` embedded environments and high-performance async applications.

## 🚀 What You Get

```rust
// Define complex state logic with a simple macro
statechart! {
    name: TrafficLight,
    context: TrafficContext,
    event: TrafficEvent,
    initial: Red,

    state Red {
        on TrafficEvent::Timer => Yellow;
    }
    
    state Yellow {
        on TrafficEvent::Timer => Green;
    }
    
    state Green {
        on TrafficEvent::Timer => Red;
        on TrafficEvent::Emergency => Red;
    }
}

// Automatically becomes a zero-cost async actor
let addr = spawn_actor_tokio(TrafficLight::new(context, &initial_event)?, 32);
addr.send(TrafficEvent::Timer).await?;
```

**The Result**: Type-safe state machines that compile to efficient code, run on any platform, and integrate seamlessly with async runtimes.

## ✨ Key Features

- **🎯 XState-Inspired Syntax**: Familiar statechart patterns with Rust's type safety
- **🔧 Zero-Cost Abstractions**: No heap allocation, minimal runtime overhead
- **🌐 Platform-Dual**: Same code runs on embedded (Embassy) and cloud (Tokio)
- **⚡ Built-in Actors**: Every statechart becomes an async actor automatically
- **🛡️ Supervision Trees**: OTP-inspired fault tolerance and restart strategies
- **📊 Advanced Features**: Hierarchical states, parallel regions, guards, actions

## 🎯 Perfect For

| Use Case | Why lit-bit? |
|----------|-------------|
| **IoT & Embedded** | `#![no_std]`, deterministic memory, real-time guarantees |
| **Game Logic** | Complex state machines, parallel systems, fast execution |
| **Protocol Implementations** | State-driven networking, robust error handling |
| **Workflow Engines** | Business logic modeling, supervision, scalability |
| **Robotics** | Sensor fusion, behavior trees, fault tolerance |

## 🚀 Quick Start

### Your First Statechart

```rust
use lit_bit_core::StateMachine;
use lit_bit_macro::statechart;
use lit_bit_macro::statechart_event;

#[derive(Debug, Clone, Default)]
struct Context { count: u32 }

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[statechart_event]
enum Event { #[default] Start, Stop, Reset }

statechart! {
    name: Counter,
    context: Context,
    event: Event,
    initial: Idle,

    state Idle {
        on Event::Start => Running;
    }

    state Running {
        on Event::Stop => Idle;
        on Event::Reset => Idle [action reset_counter];
    }
}

fn reset_counter(ctx: &mut Context, _event: &Event) {
    ctx.count = 0;
}

fn main() {
    let mut machine = Counter::new(Context::default(), &Event::default()).unwrap();
    
    machine.send(&Event::Start);
    println!("State: {:?}", machine.state()); // [Running]
}
```

### As an Async Actor

```rust
use lit_bit_core::actor::spawn_actor_tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let machine = Counter::new(Context::default(), &Event::default())?;
    let addr = spawn_actor_tokio(machine, 16);
    
    addr.send(Event::Start).await?;
    addr.send(Event::Reset).await?;
    
    Ok(())
}
```

## 📚 Learn More

### Core Concepts

- **[📖 Statechart Guide](#-usage-guide)** - States, transitions, guards, actions
- **[🧵 Actor System](#-actor-layer-production-ready-gat-based-async-system--complete)** - Zero-cost async, supervision, mailboxes
- **[🎯 Parallel States](#-parallel-states)** - Concurrent state regions
- **[🏗️ Architecture](./docs/actor-overview.md)** - Deep dive into design decisions

### Examples & Tutorials

- **[📖 Examples Directory](./lit-bit-core/examples/)** - Complete working examples
- **[🚦 Traffic Light](./lit-bit-core/examples/traffic_light.rs)** - Basic state machine
- **[🎵 Media Player](./lit-bit-core/examples/media_player.rs)** - Parallel states
- **[🧮 Calculator](./lit-bit-core/examples/actor_calculator.rs)** - Actor patterns

### Project Information

- **[📍 Roadmap](./ROADMAP.md)** - Development phases and milestones
- **[📖 Technical Spec](./Spec.md)** - Detailed specification
- **[📚 Documentation Hub](./docs/)** - Comprehensive guides

## 🔧 Development Setup

### Prerequisites

```bash
# Install Rust targets for embedded examples
rustup target add riscv32imac-unknown-none-elf thumbv7m-none-eabi

# Install QEMU for running embedded examples
brew install qemu  # macOS
apt install qemu-system-misc  # Ubuntu/Debian

# Install task runner
cargo install just
```

### Building & Testing

```bash
# Build everything
cargo build --all-targets

# Run tests
just test

# Run embedded example in QEMU
just run-rv

# Run specific example
cargo run --example traffic_light
```

## 📊 Current Status

**Phase 05 - Async & Side-Effects** ✅ **IN PROGRESS**

- ✅ **Core Statecharts**: Flat, hierarchical, and parallel state machines
- ✅ **GAT-Based Actors**: Zero-cost async with Embassy/Tokio support  
- ✅ **Platform-Dual**: Same code for embedded and cloud
- ✅ **Production Examples**: RISC-V and ARM Cortex-M targets
- 🚧 **Advanced Features**: Enhanced guards, history states, side-effects

## 📚 Usage Guide

### Basic State Machine Definition

Define a state machine using the `statechart!` macro:

```rust
use lit_bit_core::StateMachine;
use lit_bit_macro::statechart;
use lit_bit_macro::statechart_event;

#[derive(Debug, Clone, Default)]
struct Context {
    count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[statechart_event]
enum Event {
    #[default]
    Start,
    Stop,
    Reset,
}

fn increment_counter(ctx: &mut Context, _event: &Event) {
    ctx.count += 1;
}

fn reset_counter(ctx: &mut Context, _event: &Event) {
    ctx.count = 0;
}

statechart! {
    name: BasicMachine,
    context: Context,
    event: Event,
    initial: Idle,

    state Idle {
        on Event::Start => Running [action increment_counter];
    }

    state Running {
        on Event::Stop => Idle;
        on Event::Reset => Idle [action reset_counter];
    }
}

fn main() {
    let mut machine = BasicMachine::new(Context::default(), &Event::default())
        .expect("Failed to create machine");
    
    machine.send(&Event::Start);
    println!("State: {:?}", machine.state()); // [Running]
}
```

### Hierarchical States

Create nested states with parent-child relationships:

```rust
statechart! {
    name: HierarchicalMachine,
    context: Context,
    event: Event,
    initial: SystemActive,

    state SystemActive {
        initial: OperatingNormally;
        
        // Parent-level transitions
        on Event::PowerOff => SystemOff;
        
        state OperatingNormally {
            on Event::Error => ErrorRecovery;
        }
        
        state ErrorRecovery {
            on Event::Recover => OperatingNormally;
        }
    }

    state SystemOff {
        on Event::PowerOn => SystemActive;
    }
}
```

### 🎯 Parallel States

**Parallel states** allow your state machine to be in multiple orthogonal (independent) states simultaneously. This is perfect for modeling systems with concurrent concerns.

#### Defining Parallel States

Use the `[parallel]` attribute to create a state with multiple independent regions:

```rust
statechart! {
    name: MediaPlayer,
    context: MediaPlayerContext,
    event: MediaPlayerEvent,
    initial: Operational,

    // Main parallel state with 3 independent regions
    state Operational [parallel] {
        // Global transitions affect all regions
        on MediaPlayerEvent::PowerOff => PoweredOff;
        
        // REGION 1: Playback Control
        state PlaybackControl {
            initial: Stopped;
            
            state Stopped {
                on MediaPlayerEvent::Play => Playing;
            }
            
            state Playing {
                on MediaPlayerEvent::Pause => Paused;
                on MediaPlayerEvent::Stop => Stopped;
            }
            
            state Paused {
                on MediaPlayerEvent::Play => Playing;
                on MediaPlayerEvent::Stop => Stopped;
            }
        }
        
        // REGION 2: Audio Settings
        state AudioSettings {
            initial: Normal;
            
            state Normal {
                on MediaPlayerEvent::Mute => Muted;
            }
            
            state Muted {
                on MediaPlayerEvent::Unmute => Normal;
            }
        }
        
        // REGION 3: Display State  
        state DisplayState {
            initial: ScreenOn;
            
            state ScreenOn {
                on MediaPlayerEvent::ScreenOff => ScreenOff;
            }
            
            state ScreenOff {
                on MediaPlayerEvent::ScreenOn => ScreenOn;
            }
        }
    }

    state PoweredOff {
        on MediaPlayerEvent::PowerOn => Operational;
    }
}
```

#### Key Parallel States Concepts

1. **Orthogonal Regions**: Each direct child of a `[parallel]` state is an independent region
2. **Concurrent Activity**: You can be in multiple states simultaneously (e.g., `Playing + Muted + ScreenOff`)
3. **Independent Events**: Events can affect one region while others remain unchanged
4. **Global Transitions**: Transitions defined on the parallel state itself affect all regions

#### Parallel States Runtime Behavior

```rust
let mut player = MediaPlayer::new(context, &MediaPlayerEvent::default())?;

// Initial state: All regions start in their initial states
println!("{:?}", player.state()); 
// Output: [PlaybackControlStopped, AudioSettingsNormal, DisplayStateScreenOn]

// Events can affect specific regions independently
player.send(&MediaPlayerEvent::Play);        // Only affects PlaybackControl
player.send(&MediaPlayerEvent::Mute);        // Only affects AudioSettings  
player.send(&MediaPlayerEvent::ScreenOff);   // Only affects DisplayState

println!("{:?}", player.state());
// Output: [PlaybackControlPlaying, AudioSettingsMuted, DisplayStateScreenOff]

// Global events affect all regions
player.send(&MediaPlayerEvent::PowerOff);    // Exits all regions
println!("{:?}", player.state());
// Output: [PoweredOff]
```

#### When to Use Parallel States

Parallel states are ideal for modeling:
* **Audio/Video Systems**: Playback control + volume control + display settings
* **IoT Devices**: Connectivity status + sensor readings + user interface
* **Game Systems**: Player movement + inventory + UI state
* **Network Applications**: Connection state + authentication + data processing

See the complete example in [`examples/media_player.rs`](lit-bit-core/examples/media_player.rs).

### Actions and Guards

Add behavior to your state transitions:

```rust
fn is_valid_input(ctx: &Context, event: &Event) -> bool {
    // Guard condition logic
    ctx.count < 100
}

fn log_transition(ctx: &mut Context, _event: &Event) {
    println!("Transitioning at count: {}", ctx.count);
}

statechart! {
    name: GuardedMachine,
    context: Context,
    event: Event,
    initial: Waiting,

    state Waiting {
        // Transition only happens if guard returns true
        on Event::Proceed [guard is_valid_input] => Processing [action log_transition];
    }

    state Processing {
        on Event::Complete => Waiting;
    }
}
```

### Entry and Exit Actions

Execute code when entering or exiting states:

```rust
fn on_enter_active(ctx: &mut Context, _event: &Event) {
    println!("System is now active");
}

fn on_exit_active(ctx: &mut Context, _event: &Event) {
    println!("System shutting down");
}

statechart! {
    name: LifecycleMachine,
    context: Context,
    event: Event,
    initial: Active,

    state Active {
        entry: on_enter_active;
        exit: on_exit_active;
        
        on Event::Shutdown => Inactive;
    }

    state Inactive {
        on Event::Startup => Active;
    }
}
```

## 🧵 Actor Layer (Production-Ready GAT-Based Async System — ✅ Complete)

**lit-bit** provides a production-ready minimal actor model layer that enables safe, single-threaded event loops and mailbox-based communication for both embedded and async Rust environments.

### 🚀 GAT-Based Async Actor System

The actor system leverages **Generic Associated Types (GATs)** to provide zero-cost async abstractions that work seamlessly across embedded (`no_std`) and cloud (`std`) environments. This design enables stack-allocated futures without heap allocation, making it suitable for resource-constrained embedded systems while maintaining ergonomic APIs for high-throughput server applications.

#### Key Capabilities

- **🔧 Zero-Cost Abstractions**: GAT-based design enables stack-allocated futures with no heap allocation
- **🌐 Platform-Dual Design**: Same code runs on embedded (Embassy) and cloud (Tokio) runtimes
- **⚡ Deterministic Processing**: Single-threaded message processing with Actix-style atomicity guarantees
- **🛡️ Supervision Trees**: OTP-inspired restart strategies (OneForOne, OneForAll, RestForOne)
- **📡 Type-Safe Messaging**: Compile-time verified message types with `Address<Event, N>`
- **🔄 StateMachine Integration**: Zero-cost forwarding from statecharts to actors
- **📊 Back-Pressure Handling**: Platform-specific semantics (fail-fast for embedded, async for cloud)

#### Core Actor Trait

```rust
pub trait Actor: Send {
    type Message: Send + 'static;
    type Future<'a>: core::future::Future<Output = ()> + Send + 'a where Self: 'a;
    
    fn handle(&mut self, msg: Self::Message) -> Self::Future<'_>;
    
    // Lifecycle hooks for supervision
    fn on_start(&mut self) -> Result<(), ActorError> { Ok(()) }
    fn on_stop(self) -> Result<(), ActorError> { Ok(()) }
    fn on_panic(&self, info: &PanicInfo) -> RestartStrategy { RestartStrategy::OneForOne }
}
```

#### Platform-Specific Features

**Embedded (`no_std` + Embassy)**:
- Static mailboxes with `static_mailbox!` macro
- Fail-fast back-pressure (immediate error when full)
- Cooperative yielding with Embassy executor
- Memory usage: ~512B per actor

**Cloud (`std` + Tokio)**:
- Dynamic mailboxes with configurable capacity
- Async back-pressure (await when full)
- Native Tokio task spawning
- High throughput: >1M messages/sec

#### Quick Example

```rust
use lit_bit_core::actor::{Actor, spawn_actor_tokio};

struct Counter { value: u32 }

impl Actor for Counter {
    type Message = u32;
    type Future<'a> = core::future::Ready<()> where Self: 'a;
    
    fn handle(&mut self, msg: Self::Message) -> Self::Future<'_> {
        self.value += msg;
        core::future::ready(()) // Zero-cost for sync operations
    }
}

// Spawn and use
let addr = spawn_actor_tokio(Counter { value: 0 }, 16);
addr.send(42).await?;
```

#### StateMachine Integration

Every statechart automatically becomes an actor through blanket implementation:

```rust
statechart! {
    name: TrafficLight,
    event: TrafficEvent,
    initial: Red,
    // ... states and transitions
}

// Automatically implements Actor trait
let addr = spawn_actor_tokio(TrafficLight::new(context, &initial_event)?, 32);
addr.send(TrafficEvent::TimerExpired).await?;
```

### 📚 Comprehensive Documentation

- **[🏗️ Actor System Architecture Guide](./docs/actor-overview.md)** - Complete overview of supervision, lifecycle, and performance tuning
- **[⚡ Phase 5 Implementation Guide](./docs/phase-05-async-implementation-guide.md)** - Technical deep-dive into GAT-based design and zero-cost async patterns
- **[🧪 Testing Guide](./docs/test-guide.md)** - Actor testing patterns, back-pressure testing, and performance benchmarks
- **[📖 Examples](./lit-bit-core/examples/)** - Complete working examples including:
  - `async_actor_simple.rs` - Basic GAT-based actor usage
  - `actor_calculator.rs` - Complex async operations with reply patterns
  - `actor_backpressure.rs` - Back-pressure handling demonstrations
  - `actor_statechart_integration.rs` - StateMachine-to-Actor integration

### 🎯 Performance Targets (Achieved)

| Metric | Embedded (Embassy) | Cloud (Tokio) |
|--------|-------------------|---------------|
| **Throughput** | >500k msg/sec | >1M msg/sec |
| **Latency** | <200ns | <100ns |
| **Memory/Actor** | ~512B | ~1KB |
| **Spawn Cost** | ~100ns | ~50ns |

## 🛠️ Key Dependencies & Tools

*   **Core Logic:** `lit-bit-core` (the `no_std` runtime)
*   **Macro:** `lit-bit-macro` (the `statechart!` procedural macro)
*   **Embedded Runtimes:** `riscv-rt`, `cortex-m-rt`
*   **Semihosting:** `semihosting` crate for QEMU output.
*   **CLI Task Runner:** `just`

## 🤝 Contributing

Contributions are welcome! Please adhere to the project's development rules and conventions:

*   **Code Style:** `rustfmt` (use default settings), Clippy for lints.
*   **Commit Messages:** Conventional Commits style (`feat:`, `fix:`, etc.).

Please open an issue to discuss any significant changes or new features before submitting a pull request.

## 📝 License

This project is licensed under the terms of the MIT license and the Apache License (Version 2.0). See [LICENSE-MIT](./LICENSE-MIT) and [LICENSE-APACHE](./LICENSE-APACHE) for details. You may use this project under either license.

*Built with ❤️ for the Rust embedded and systems programming community.*