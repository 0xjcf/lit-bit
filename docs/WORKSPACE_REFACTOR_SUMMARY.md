# Workspace Refactor & Safe Static Mailbox Implementation

## Overview

This document summarizes the comprehensive workspace refactor completed for the lit-bit project, implementing the recommendations from the research report to create a clean separation between embedded and host builds, add safe static mailbox APIs, and prepare for Phase 05 async work.

## 1. Workspace Structure Refactor

### 1.1 New Crate Organization

| Crate | Purpose | Publish | Dependencies |
|-------|---------|---------|--------------|
| `lit-bit-core` | Core statechart & actor library | ✅ | Minimal, no_std by default |
| `lit-bit-macro` | Procedural macros | ✅ | Minimal |
| `lit-bit-cli` | CLI tooling | ✅ | Host-only |
| `lit-bit-tests` | Integration & property tests | ❌ | Heavy test deps (std-only) |
| `lit-bit-bench` | Performance benchmarks | ❌ | Criterion, Iai-Callgrind |

### 1.2 Workspace Configuration

```toml
[workspace]
resolver = "2"
members = [
    "lit-bit-cli",
    "lit-bit-core", 
    "lit-bit-macro",
    "lit-bit-tests",
    "lit-bit-bench"
]
exclude = ["xtask"]
default-members = ["lit-bit-core", "lit-bit-macro", "lit-bit-cli"]
```

**Key Benefits:**
- `cargo build` only builds production crates
- Test/bench crates are isolated with heavy dependencies
- Resolver 2 prevents feature bleeding between crates

## 2. Dependency Isolation

### 2.1 Target-Specific Dev Dependencies

The core crate now uses target-specific dev dependencies to prevent heavy host dependencies from leaking into embedded builds:

```toml
# Target-specific dev dependencies to prevent heavy deps from leaking into embedded builds
[target.'cfg(not(target_os = "none"))'.dev-dependencies]
tokio = { version = "1.42", features = ["macros", "rt", "rt-multi-thread", "sync", "time"] }
criterion = { version = "0.5", features = ["html_reports"] }
proptest = "1.4"

# Minimal dev dependencies for embedded targets
[target.'cfg(target_os = "none")'.dev-dependencies]
panic-halt = "1.0.0"
```

### 2.2 Test Migration

All integration tests have been migrated from `lit-bit-core/tests/` to the new `lit-bit-tests` crate, ensuring:
- Heavy test dependencies don't affect embedded builds
- Tests can use std features freely
- Property-based testing with proptest
- Async testing with tokio

## 3. Safe Static Mailbox API

### 3.1 Existing Implementation

The project already had a robust `static_mailbox!` macro that provides zero-unsafe mailbox creation:

```rust
// Create a mailbox for u32 messages with capacity 16
let (producer, consumer) = static_mailbox!(MY_QUEUE: u32, 16);

// With memory placement attribute
let (tx, rx) = static_mailbox!(
    #[link_section = ".sram2"]
    FAST_QUEUE: MyMessage, 32
);
```

### 3.2 Safety Features

- Uses atomic flags to prevent double-initialization
- Leverages `heapless::spsc::Queue` for zero-allocation operation
- Provides compile-time capacity checking
- Supports memory placement attributes for embedded optimization

## 4. Feature Flag Architecture

### 4.1 Current Features

| Feature | Default | Purpose |
|---------|---------|---------|
| `std` | ❌ | Enables Tokio mailbox & heap features |
| `async` | ❌ | Async trait support (requires std) |
| `embassy` | ❌ | Embassy executor integration |
| `diagram` | ❌ | Statechart visualization |

### 4.2 Conditional Compilation

The codebase uses extensive conditional compilation to ensure embedded builds only include necessary code:

```rust
#[cfg(not(feature = "std"))]
pub type Inbox<T, const N: usize> = heapless::spsc::Consumer<'static, T, N>;

#[cfg(feature = "std")]
pub type Inbox<T, const N: usize> = tokio::sync::mpsc::Receiver<T>;
```

## 5. Automation & CI

### 5.1 Xtask Implementation

Created a comprehensive `xtask` automation tool with commands:

- `xtask ci --target <target>` - Run CI for specific targets
- `xtask check-all` - Check all targets (x86_64, ARM, RISC-V)
- `xtask test` - Run all tests
- `xtask bench --smoke` - Quick benchmark check

### 5.2 Justfile Integration

Updated the justfile to use xtask for consistent automation:

```bash
# Run CI checks for all targets
just ci

# Run CI for a specific target  
just ci-target thumbv7m-none-eabi

# Run all tests via xtask
just test-all

# Run benchmarks in smoke mode
just bench-smoke
```

## 6. Build Verification

### 6.1 Embedded Targets

✅ **Cortex-M (thumbv7m-none-eabi)**
```bash
cargo check --target thumbv7m-none-eabi -p lit-bit-core --no-default-features
```

✅ **RISC-V (riscv32imac-unknown-none-elf)**
```bash
cargo check --target riscv32imac-unknown-none-elf -p lit-bit-core --no-default-features
```

### 6.2 Host Targets

✅ **Full workspace build**
```bash
cargo build --workspace
```

✅ **Test suite**
```bash
cargo test -p lit-bit-tests
# 28 tests passed
```

✅ **Benchmark compilation**
```bash
cargo check -p lit-bit-bench
```

## 7. Performance Benchmarks

### 7.1 Benchmark Structure

The `lit-bit-bench` crate includes:

- **Throughput benchmarks**: Statechart transition performance
- **Latency benchmarks**: Actor mailbox send/receive timing
- **Memory benchmarks**: Allocation tracking and zero-allocation verification
- **Iai-Callgrind integration**: Instruction-level performance analysis

### 7.2 Benchmark Utilities

- Custom allocator tracking for memory usage analysis
- Realistic workload generators
- Configurable benchmark parameters
- Statistical analysis of results

## 8. Next Steps (Phase 05)

The refactor sets up the foundation for Phase 05 async implementation:

1. **Week 1**: Async design finalization
2. **Week 1.5**: Baseline performance benchmarks
3. **Week 2**: Zero-cost async handlers with GAT
4. **Week 2.5**: Async TestKit and timeout utilities
5. **Week 3**: Performance validation and documentation

## 9. Key Achievements

✅ **Zero std dependencies leak into embedded builds**
✅ **Safe static mailbox API with zero unsafe code for users**
✅ **Comprehensive test suite isolated from core crate**
✅ **Performance benchmarking infrastructure**
✅ **Multi-target CI automation**
✅ **Clean workspace organization**
✅ **Resolver 2 feature isolation**

## 10. Verification Commands

```bash
# Test embedded builds (no std dependencies)
just ci-target thumbv7m-none-eabi
just ci-target riscv32imac-unknown-none-elf

# Test host builds (full features)
just ci-target x86_64-unknown-linux-gnu

# Run all tests
just test-all

# Check benchmarks
just bench-smoke

# Full CI matrix
just ci
```

This refactor successfully implements all recommendations from the research report and provides a solid foundation for Phase 05 async development while maintaining the project's commitment to zero-cost embedded operation. 