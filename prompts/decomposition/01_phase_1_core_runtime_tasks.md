# Phase 1 · Core Runtime — Task Decomposition

> Derived from the [Phase 1 Checklist](../phases/02-development/00_checklist.md). Break these down further into actionable steps or PRs as needed.

## Key Deliverables from Checklist

1.  **`StateMachine` trait & flat FSM runtime implemented (`#![no_std]` default)**
    *   [ ] Task: Define initial `State`, `Event`, `Context` placeholder types in `src/core/mod.rs`.
    *   [x] Task: Define `Transition<S, E>` and `MachineDefinition<S, E, C>` structs in `src/core/mod.rs` using `&'static [Transition<S,E>]` for `no_std` compatibility.
    *   [x] Task: Define `Runtime<S, E, C>` struct in `src/core/mod.rs` to hold current state, context, and machine definition.
    *   [x] Task: Implement `StateMachine` trait for `Runtime` with basic `send`, `state`, `context`, `context_mut` methods.
    *   [ ] Task: Implement basic entry/exit action and guard placeholder logic in `Runtime::send()`.
    *   [ ] Task: Ensure core path can compile with `#![no_std]` (no `alloc` unless `alloc` feature is present).

2.  **Exhaustive `match` compile-time enforcement for all states/events**
    *   [ ] Task: Design strategy for ensuring exhaustive matches (likely handled by macro generation later, but note requirement).
    *   [ ] Task: Placeholder for any runtime checks if compile-time is not fully feasible initially for manually defined machines.

3.  **`traffic_light` demo builds & runs on**
    *   **Native x86_64**
        *   [ ] Task: Create `examples/traffic_light.rs` with basic structure.
        *   [ ] Task: Define `TrafficLightState`, `TrafficLightEvent`, `TrafficLightContext` for the demo.
        *   [ ] Task: Manually define a `MachineDefinition` for the traffic light in the example.
        *   [ ] Task: Instantiate and run the `Runtime` with the traffic light definition, printing state changes.
        *   [ ] Task: Ensure `cargo run --example traffic_light` works.
    *   **RISC-V QEMU (`riscv32imac-unknown-none-elf`)**
        *   [ ] Task: Set up target `riscv32imac-unknown-none-elf` for the project.
        *   [ ] Task: Adapt `traffic_light` example to be `no_std` compatible for embedded execution.
        *   [ ] Task: Implement a simple output mechanism for QEMU (e.g., semihosting, UART) to observe state changes.
        *   [ ] Task: Create a build script or `Embed.toml` for easy compilation and running in QEMU.
        *   [ ] Task: Verify demo runs on `riscv32` via QEMU.

4.  **Release build flash report ≤ 1 KB (thumb-v7m size-check)**
    *   [ ] Task: Set up target `thumbv7m-none-eabi` (or similar Cortex-M target).
    *   [ ] Task: Adapt `traffic_light` demo for this target (minimal version).
    *   [ ] Task: Use `cargo-binutils` (e.g., `cargo size --release --target thumbv7m-none-eabi --example traffic_light`) to check flash size.
    *   [ ] Task: Optimize code if necessary to meet the ≤1KB target for the demo.

5.  **100% unit-test coverage for core runtime**
    *   [ ] Task: Add unit tests to `src/core/mod.rs` for the `Runtime`.
    *   [ ] Task: Test basic state transitions.
    *   [ ] Task: Test event handling (event causes transition, event ignored).
    *   [ ] Task: Test context access.
    *   [ ] Task: (Later) Test entry/exit actions, guards.
    *   [ ] Task: Set up `cargo-tarpaulin` or similar for coverage measurement.
    *   [ ] Task: Achieve 100% line coverage for the implemented core runtime logic.

6.  **No heap allocations in core path; Clippy `pedantic` passes**
    *   [x] Task: Confirmed `MachineDefinition` uses `&'static []` (no `alloc` for definition).
    *   [ ] Task: Review `Runtime` and other core logic to ensure no heap allocations occur without `alloc` or `std` features.
    *   [ ] Task: Run `cargo clippy --all-targets --all-features -- -W clippy::pedantic -A clippy::missing_errors_doc -A clippy::missing_panics_doc -A clippy::module_name_repetitions` (or similar adjusted set) and fix lints.

---

*Use this list to guide implementation during Phase 1. Create specific GitHub issues or PRs for larger tasks.* 