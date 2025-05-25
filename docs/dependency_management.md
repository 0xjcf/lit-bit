# Dependency Management Strategy

## Overview

This project follows Rust best practices for managing dependencies in a workspace that supports both embedded (`no_std`) and host (`std`) environments. We use a **two-crate workspace pattern** that cleanly separates concerns and avoids forcing unnecessary dependencies on library users.

## Workspace Structure

```
lit-bit/
├── lit-bit-core/     # Lean library crate (no_std by default)
├── lit-bit-cli/      # Binary crate with heavier dependencies (std)
├── lit-bit-macro/    # Procedural macros
└── Cargo.toml        # Workspace configuration
```

## Design Principles

### 1. **Two-Crate Pattern** (Recommended by Rust Community)

Instead of using complex feature gating within a single crate, we split functionality:

- **`lit-bit-core`**: The reusable library with minimal dependencies
- **`lit-bit-cli`**: Binary tools that can use heavier dependencies

This approach is recommended in:
- Rust Forum discussions on library+binary design
- Axo.dev blog post "It's a library AND a binary" by Gankra
- Community consensus on Reddit and Users Forum

### 2. **Feature-Gated Dependencies in Core**

The core library uses optional dependencies with clear feature flags:

```toml
[dependencies]
# Always available (no_std compatible)
heapless = { version = "0.8.0", default-features = false }

# Optional dependencies behind features
serde = { version = "1.0", optional = true }
tokio = { version = "1.42", optional = true }

[features]
default = []  # Keep minimal for embedded users
std = ["dep:anyhow", "dep:thiserror", "dep:tokio"]
async = ["std", "dep:async-trait", "dep:futures"]
diagram = ["dep:serde"]
```

### 3. **CLI Crate Always Enables Needed Features**

The CLI crate depends on the core with required features:

```toml
[dependencies]
lit-bit-core = { path = "../lit-bit-core", features = ["std", "async", "diagram"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

## Benefits of This Approach

### ✅ **Clean Library Experience**
- Library users get minimal dependencies by default
- No need for `--no-default-features` gymnastics
- Clear feature opt-in for additional functionality

### ✅ **Simple Binary Development**
- CLI developers don't need to remember feature flags
- All needed dependencies are always available
- No `required-features` complexity

### ✅ **Excellent CI/CD**
- Test core library on embedded targets without std
- Test CLI on host targets with full feature set
- Clear separation of concerns in build matrix

### ✅ **User-Friendly Installation**
- `cargo install lit-bit-cli` just works (no feature flags needed)
- Library users add `lit-bit-core` and get lean dependencies
- No silent binary installation failures

## CI Strategy

Our CI tests multiple configurations to ensure robustness:

### Core Library Testing
```bash
# Test minimal no_std build for embedded
cargo check --target thumbv7m-none-eabi --no-default-features -p lit-bit-core

# Test with features on host
cargo test --features "std,async,diagram" -p lit-bit-core
```

### CLI Testing
```bash
# CLI always builds with full features
cargo build --all-features -p lit-bit-cli
cargo test -p lit-bit-cli
```

### Workspace Testing
```bash
# Ensure all crates work together
cargo check --all-targets --workspace
cargo clippy --all-targets --workspace -- -D warnings
```

## Comparison with Single-Crate Approach

| Aspect | Single Crate + Features | Two-Crate Workspace |
|--------|------------------------|---------------------|
| Library UX | Requires `--no-default-features` | Clean by default |
| Binary UX | Requires `--features` flags | Just works |
| CI Complexity | Feature matrix testing | Simple per-crate testing |
| Install UX | Silent failures without features | Always works |
| Maintenance | Feature flag coordination | Clear separation |

## Target-Specific Dependencies

We use Cargo's target-specific dependency tables for platform-specific code:

```toml
# ARM Cortex-M targets
[target.'cfg(target_arch = "arm")'.dependencies]
cortex-m = "0.7.7"
cortex-m-rt = "0.7.5"

# RISC-V targets  
[target.'cfg(target_arch = "riscv32")'.dependencies]
riscv-rt = { version = "0.14.0", features = ["single-hart"] }
riscv = "0.13.0"
```

## Future Considerations

- **Workspace Features**: Could be used for coordinated feature enabling across crates
- **Binary Distribution**: Consider separate repositories if CLI becomes large
- **Plugin Architecture**: Additional crates for specialized functionality

## References

- [Cargo Book - Features](https://doc.rust-lang.org/cargo/reference/features.html)
- [Cargo Book - Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [Rust Forum - Dependencies from library and binary crate](https://users.rust-lang.org/t/dependencies-from-library-and-binary-crate-in-a-package/22147)
- [Axo.dev - It's a library AND a binary](https://blog.axo.dev/2024/01/29/its-a-library-and-a-binary) 