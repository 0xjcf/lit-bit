# lit-bit: A `#![no_std]` Statechart Library for Rust

[![Heap/Unsafe-Free CI](https://img.shields.io/badge/heap--unsafe--free-checked-brightgreen?logo=rust&label=Heap%2FUnsafe%20Scan)](https://github.com/0xjcf/lit-bit/actions)

![CodeRabbit Pull Request Reviews](https://img.shields.io/coderabbit/prs/github/0xjcf/lit-bit)


**`lit-bit` is a lightweight, procedural macro-driven statechart library for Rust, designed for correctness, ease of use, and suitability for embedded systems (`#![no_std]`) as well as general applications.**

It aims to provide a similar developer experience to XState but within the Rust ecosystem, focusing on compile-time safety and minimal resource footprint for bare-metal targets like RISC-V and ARM Cortex-M.

## 📋 Project Vision & Documentation

- **[📍 Roadmap](./ROADMAP.md)** - Project phases, milestones, and future development plans
- **[📖 Technical Specification](./Spec.md)** - Detailed technical specification and design decisions
- **[📚 Documentation Hub](./docs/)** - Comprehensive guides, tutorials, and architectural overviews

## Current Status

*   **Phase:** 04 - Minimal Actor Layer ✅ **COMPLETED** | Next: Phase 05 - Async & Side-Effects (planning)
*   Core runtime for flat, hierarchical, and parallel state machines is functional.
*   Procedural macro (`statechart!`) for defining state machines is operational for flat, hierarchical, and parallel structures.
*   **Minimal actor system and mailbox integration is complete** with full Embassy/Tokio support.
*   Examples for RISC-V (QEMU) and Cortex-M (QEMU/hardware) are available.
*   **Actor layer provides**: Type-safe addresses, hierarchical spawning, supervision trees, platform-dual mailboxes.


## ✨ Features

*   **`#![no_std]` by default:** Suitable for bare-metal embedded applications.
*   **`statechart!` Macro:** Define complex state machines with a clear, XState-inspired syntax.
    *   States, events, transitions, entry/exit actions, initial states.
    *   Hierarchical states.
    *   Parallel states (supported).
    *   Comprehensive event type support (both Copy and non-Copy types like `String`, `Vec`, custom structs).
    *   (Planned: History states, enhanced guards, advanced context/data management).
*   **Compile-Time Safety:** Leverage Rust's type system to catch errors at compile time.
*   **Minimal Footprint:** Designed to be lightweight in terms of code size and RAM usage for embedded targets.
*   **Dual Target Examples:**
    *   RISC-V (`riscv32imac-unknown-none-elf`) via QEMU.
    *   ARM Cortex-M (`thumbv7m-none-eabi`) (setup for QEMU/hardware).
*   **Test-Driven:** Extensive unit and integration tests.

## 🚀 Getting Started

### Prerequisites

*   **Rust Toolchain:** Install via [rustup.rs](https://rustup.rs/). Ensure you have the `riscv32imac-unknown-none-elf` and `thumbv7m-none-eabi` targets installed:
    ```bash
    rustup target add riscv32imac-unknown-none-elf
    rustup target add thumbv7m-none-eabi
    ```
*   **QEMU:** Required for running the RISC-V examples. Install via your system's package manager (e.g., `brew install qemu` on macOS, `apt install qemu-system-misc` on Debian/Ubuntu).
*   **`just`:** A command runner. Install via `cargo install just` or your package manager.

### Building the Project

```bash
# Build all crates in the workspace
cargo build --all-targets

# Build for a specific target (e.g., RISC-V)
cargo build --target riscv32imac-unknown-none-elf
```

### Running Examples

The project uses `just` to simplify common tasks.

*   **Run RISC-V `traffic_light` example in QEMU:**
    ```bash
    just run-rv
    ```
    This will compile the example and launch it using QEMU with semihosting enabled for console output.

*   **(Planned/WIP) Run Cortex-M `traffic_light` example:**
    ```bash
    just run-cm 
    ```

### Running Tests

```bash
# Run tests for all workspace crates
just test

# Run tests for a specific crate (e.g., lit-bit-core)
just test-core
```

## 📚 Usage Guide

### Basic State Machine Definition

Define a state machine using the `statechart!` macro:

```rust
use lit_bit_core::StateMachine;
use lit_bit_macro::statechart;

#[derive(Debug, Clone, Default)]
struct Context {
    count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
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

## 🧵 Actor Layer (Phase 04: Minimal Actor System — ✅ Complete)

**lit-bit** provides a production-ready minimal actor model layer that enables safe, single-threaded event loops and mailbox-based communication for both embedded and async Rust environments.

*Built with ❤️ for the Rust embedded and systems programming community.*