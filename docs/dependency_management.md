# Dependency Management Strategy

## Overview

This project follows Rust best practices for managing dependencies in a workspace that supports both embedded (`no_std`) and host (`std`) environments. We use a **multi-crate workspace pattern** with development isolation that cleanly separates concerns and avoids forcing unnecessary dependencies on library users.

## Workspace Structure

```
lit-bit/
├── lit-bit-core/     # 📦 Core library (no_std by default, minimal deps)
├── lit-bit-macro/    # 🔧 Procedural macros (proc-macro crate)
├── lit-bit-cli/      # 🖥️  CLI tools (std, heavier dependencies)
├── lit-bit-tests/    # 🧪 Integration tests (publish = false)
├── lit-bit-bench/    # ⚡ Benchmarks (publish = false, heavy deps)
├── xtask/            # 🛠️  Build automation (excluded from workspace)
└── Cargo.toml        # 🏗️  Workspace configuration
```

## Design Principles

### 1. **Development Isolation Pattern** (Advanced Workspace Design)

We use a sophisticated approach that isolates development dependencies from published crates:

- **`lit-bit-core`**: The lean, publishable library with minimal dependencies
- **`lit-bit-macro`**: Procedural macros with minimal proc-macro dependencies
- **`lit-bit-cli`**: CLI tools that can use heavier dependencies
- **`lit-bit-tests`**: Integration tests with `publish = false` (heavy test deps)
- **`lit-bit-bench`**: Benchmarks with `publish = false` (Criterion, Iai-Callgrind)
- **`xtask`**: Build automation excluded from workspace (no version constraints)

This approach prevents "dependency pollution" where test/benchmark dependencies leak into published crates.

### 2. **Feature-Gated Dependencies in Core**

The core library uses optional dependencies with clear feature flags:

```toml
[dependencies]
# Always available (no_std compatible)
heapless = { version = "0.8.0", default-features = false }
static_cell = "2.1.0"

# Optional dependencies behind features
serde = { version = "1.0", optional = true, default-features = false }
tokio = { version = "1.42", optional = true }
futures = { version = "0.3", optional = true }
async-trait = { version = "0.1", optional = true }

[features]
default = []  # Keep minimal for embedded users
std = ["dep:tokio", "heapless/std"]
async = ["std", "dep:async-trait", "dep:futures"]
diagram = ["dep:serde", "serde/derive"]
embassy = []  # Feature flag for Embassy runtime support
```

### 3. **Development Crates with Heavy Dependencies**

Development crates can use heavy dependencies without affecting published crates:

```toml
# lit-bit-tests/Cargo.toml
[package]
name = "lit-bit-tests"
publish = false  # Never published - can use any dependencies

[dependencies]
lit-bit-core = { workspace = true, features = ["std", "async"] }
tokio = { version = "1.42", features = ["full"] }
criterion = "0.5"
# ... any test dependencies without affecting core library
```

### 4. **Workspace Dependency Management**

Centralized version management for internal crates:

```toml
# Root Cargo.toml
[workspace.dependencies]
lit-bit-macro = { path = "lit-bit-macro", version = "0.1.0" }
lit-bit-core = { path = "lit-bit-core", version = "0.0.1-alpha.0" }

# Individual crates reference workspace versions
[dependencies]
lit-bit-core = { workspace = true }
```

## Benefits of This Approach

### ✅ **Clean Library Experience**
- Library users get minimal dependencies by default
- No need for `--no-default-features` gymnastics
- Clear feature opt-in for additional functionality
- Zero "dev dependency pollution" in published crates

### ✅ **Powerful Development Environment**
- Integration tests can use heavy dependencies (Tokio full features)
- Benchmarks can use Criterion, Iai-Callgrind without affecting core
- Build automation (xtask) isolated from workspace constraints
- CI can test complex scenarios without bloating library

### ✅ **Excellent CI/CD**
- Test core library on embedded targets without std
- Test integration scenarios with full feature sets
- Benchmark performance without dependency constraints
- Clear separation of concerns in build matrix

### ✅ **User-Friendly Installation**
- `cargo install lit-bit-cli` just works (no feature flags needed)
- Library users add `lit-bit-core` and get lean dependencies
- No silent binary installation failures
- Embedded users get truly minimal dependency trees

## CI Strategy

Our CI tests multiple configurations across the workspace:

### Core Library Testing
```bash
# Test minimal no_std build for embedded
cargo check --target thumbv7m-none-eabi --no-default-features -p lit-bit-core

# Test with features on host
cargo test --features "std,async,diagram" -p lit-bit-core
```

### Integration Testing
```bash
# Heavy integration tests in isolated crate
cargo test -p lit-bit-tests

# Benchmark compilation verification
cargo check -p lit-bit-bench
```

### Workspace Testing
```bash
# Ensure all publishable crates work together
cargo check --workspace --exclude lit-bit-tests --exclude lit-bit-bench

# Full linting across workspace
cargo clippy --all-targets --workspace -- -D warnings
```

### Multi-Target Validation
```bash
# Xtask automation for embedded targets
cargo xtask ci
cargo xtask check-embedded
```

## Comparison with Alternative Approaches

| Aspect | Single Crate + Features | Basic Multi-Crate | Development Isolation |
|--------|------------------------|-------------------|---------------------|
| Library UX | Requires `--no-default-features` | Clean by default | Clean by default |
| Dev Dependencies | Pollute published crate | Pollute published crate | **Isolated** |
| CI Complexity | Feature matrix testing | Simple per-crate testing | **Targeted testing** |
| Install UX | Silent failures without features | Always works | **Always works** |
| Maintenance | Feature flag coordination | Clear separation | **Clear + isolated** |
| Benchmark Deps | Affect library users | Affect library users | **No impact** |

## Target-Specific Dependencies

We use Cargo's target-specific dependency tables for platform-specific code:

```toml
# lit-bit-core/Cargo.toml
[target.'cfg(all(not(feature = "std"), target_arch = "arm"))'.dependencies]
cortex-m = "0.7.7"
cortex-m-rt = "0.7.5"

[target.'cfg(all(not(feature = "std"), target_arch = "riscv32"))'.dependencies]
riscv-rt = { version = "0.14.0", features = ["single-hart"] }
riscv = "0.13.0"
```

## Safety and Quality Assurance

### Unsafe Code Prevention
```toml
# lit-bit-core/Cargo.toml
[lints.rust]
unsafe_code = "forbid"  # Compile-time safety enforcement
```

### Heap Allocation Detection
```bash
# Automated heap safety checking
cargo run --bin heap-safety-check
```

### Custom Linting
```toml
# Custom clippy lints for actor anti-patterns
[workspace.lints.clippy]
# ... custom lint configuration
```

## Performance Optimization

### Profile Configuration
```toml
# Root Cargo.toml
[profile.release]
lto = true              # Link-time optimization
codegen-units = 1       # Single codegen unit for max optimization
strip = true            # Strip debug symbols
panic = "abort"         # Smaller binaries for embedded
```

### Feature-Specific Optimization
```toml
# Conditional optimization based on target
[profile.release.package.lit-bit-core]
opt-level = "s"  # Size optimization for embedded builds
```

## Future Considerations

### Workspace Evolution
- **Plugin Architecture**: Additional crates for specialized functionality
- **Language Bindings**: C FFI crate with `publish = false` during development
- **Documentation**: Separate mdbook crate for comprehensive guides

### Dependency Strategy
- **Workspace Features**: Coordinated feature enabling across crates
- **Version Management**: Automated dependency updates with compatibility testing
- **Supply Chain Security**: `cargo deny` integration for vulnerability scanning

## Best Practices Summary

1. **Keep published crates lean** - Use `publish = false` for development crates
2. **Isolate heavy dependencies** - Tests and benchmarks in separate crates
3. **Feature-gate optional functionality** - Clear opt-in for additional capabilities
4. **Use workspace dependencies** - Centralized version management
5. **Target-specific dependencies** - Platform-appropriate dependency selection
6. **Safety by default** - `#![forbid(unsafe_code)]` and automated checking
7. **Performance profiles** - Optimized builds for different use cases

## References

- [Cargo Book - Features](https://doc.rust-lang.org/cargo/reference/features.html)
- [Cargo Book - Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [Rust Forum - Dependencies from library and binary crate](https://users.rust-lang.org/t/dependencies-from-library-and-binary-crate-in-a-package/22147)
- [Axo.dev - It's a library AND a binary](https://blog.axo.dev/2024/01/29/its-a-library-and-a-binary)
- [Rust Embedded Book - Workspaces](https://docs.rust-embedded.org/book/start/index.html)
- [The Rust Performance Book](https://nnethercote.github.io/perf-book/) 