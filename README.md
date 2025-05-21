# lit-bit: A `#![no_std]` Statechart Library for Rust

**`lit-bit` is a lightweight, procedural macro-driven statechart library for Rust, designed for correctness, ease of use, and suitability for embedded systems (`#![no_std]`) as well as general applications.**

It aims to provide a similar developer experience to XState but within the Rust ecosystem, focusing on compile-time safety and minimal resource footprint for bare-metal targets like RISC-V and ARM Cortex-M.

## Current Status

*   **Phase:** 03 - Parallel States (In Progress)
*   Core runtime for flat and basic hierarchical state machines is functional.
*   Procedural macro (`statechart!`) for defining state machines is operational for flat and hierarchical structures.
*   Parallel state machine logic is under active development.
*   Examples for RISC-V (QEMU) and Cortex-M (QEMU/hardware) are available.


## ✨ Features

*   **`#![no_std]` by default:** Suitable for bare-metal embedded applications.
*   **`statechart!` Macro:** Define complex state machines with a clear, XState-inspired syntax.
    *   States, events, transitions, entry/exit actions, initial states.
    *   Hierarchical states.
    *   (Planned: Parallel states, history states, guards, context/data management).
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
