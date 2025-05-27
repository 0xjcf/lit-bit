# Contributing to lit-bit

Thank you for your interest in contributing to `lit-bit`! This guide will help you get started with development and understand our project conventions.

## 🚀 Quick Start

### Prerequisites

- **Rust Toolchain**: Install via [rustup.rs](https://rustup.rs/)
- **Target Platforms**: Install embedded targets
  ```bash
  rustup target add riscv32imac-unknown-none-elf
  rustup target add thumbv7m-none-eabi
  ```
- **QEMU**: For running embedded examples
- **just**: Command runner (`cargo install just`)

### Development Setup

1. **Clone and setup**:
   ```bash
   git clone https://github.com/0xjcf/lit-bit.git
   cd lit-bit
   just setup  # Install pre-commit hooks and dependencies
   ```

2. **Run tests**:
   ```bash
   just test        # All tests
   just test-core   # Core library only
   just test-embedded  # Embedded targets
   ```

3. **Check code quality**:
   ```bash
   just lint        # Clippy + rustfmt
   just check-heap  # Verify no heap usage on embedded
   ```

## 🏗️ Project Structure

```
lit-bit/
├── lit-bit-core/     # Core statechart runtime (no_std)
├── lit-bit-macro/    # Procedural macros (statechart!)
├── lit-bit-cli/      # Command-line tools
├── lit-bit-tests/    # Integration tests
├── lit-bit-bench/    # Performance benchmarks
├── docs/             # Documentation and guides
├── examples/         # Usage examples
└── prompts/          # Development planning docs
```

## 🎯 Feature Flag Matrix

Understanding our feature flags is crucial for contributing:

### Core Features
- **Default**: `#![no_std]` compatible, zero heap allocation
- **`std`**: Enables standard library features (Tokio integration)
- **`alloc`**: Enables heap allocation without full std

### Platform Features  
- **`embassy`**: Embassy executor integration (embedded async)
- **`tokio`**: Tokio runtime integration (cloud async)

### Development Features
- **`diagram`**: State machine visualization (dev/docs only)
- **`trace`**: Debug tracing and instrumentation

### Testing Matrix
```bash
# Core compatibility
cargo test --no-default-features                    # Pure no_std
cargo test --features std                          # Standard library
cargo test --features alloc                        # Heap without std

# Platform testing  
cargo test --features embassy                      # Embedded async
cargo test --features tokio                        # Cloud async

# Cross-compilation
cargo build --target thumbv7m-none-eabi            # Cortex-M
cargo build --target riscv32imac-unknown-none-elf  # RISC-V
```

## 🔧 Pre-commit Hooks

We use pre-commit hooks to maintain code quality:

### Automatic Setup
```bash
just setup  # Installs hooks automatically
```

### Manual Setup
```bash
pip install pre-commit
pre-commit install
```

### What Gets Checked
- **Rust formatting** (`rustfmt`)
- **Linting** (`clippy`)
- **Heap/unsafe scanning** (`cargo geiger`)
- **Test compilation** (embedded targets)
- **Documentation** (rustdoc warnings)

## 📝 Code Style & Conventions

### Rust Style
- **Formatting**: Use `rustfmt` with default settings
- **Linting**: Address all `clippy` warnings
- **Safety**: `#![forbid(unsafe_code)]` in core crates
- **Documentation**: All public APIs must have rustdoc

### Commit Messages
Use [Conventional Commits](https://www.conventionalcommits.org/):
```
feat: add async support for statechart actions
fix: resolve mailbox overflow in embedded targets  
docs: update actor system architecture guide
test: add integration tests for supervision trees
```

### Branch Naming
- `feat/description` - New features
- `fix/description` - Bug fixes  
- `docs/description` - Documentation updates
- `phase-XX/description` - Phase-specific work

## 🧪 Testing Guidelines

### Test Categories
1. **Unit Tests**: Test individual components in isolation
2. **Integration Tests**: Test component interactions
3. **Embedded Tests**: Verify no_std compatibility
4. **Performance Tests**: Benchmark critical paths

### Writing Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_state_transition() {
        // Test sync behavior
    }
    
    #[tokio::test]
    async fn test_async_actor() {
        // Test async behavior
    }
    
    #[test]
    #[cfg(not(feature = "std"))]
    fn test_no_std_compatibility() {
        // Test embedded compatibility
    }
}
```

### Performance Testing
- Use `criterion` for benchmarks
- Test on both x86_64 and embedded targets
- Measure memory usage with stack painting
- Validate against performance targets (see ROADMAP.md)

## 🎯 Development Phases

We follow a structured development approach:

### Current Phase: 05 - Async & Side-Effects
- **Focus**: Async integration with zero breaking changes
- **Key Areas**: GAT-based traits, Embassy/Tokio integration, timer syntax

### Contributing to Current Phase
1. Check the [Phase 05 checklist](./prompts/phases/05-async-side-effects/05_checklist.md)
2. Look for unchecked items that match your interests
3. Open an issue to discuss your approach
4. Submit a PR with tests and documentation

## 🐛 Reporting Issues

### Bug Reports
Include:
- Rust version (`rustc --version`)
- Target platform (x86_64, thumbv7m-none-eabi, etc.)
- Feature flags used
- Minimal reproduction case
- Expected vs actual behavior

### Feature Requests
- Check the [ROADMAP.md](./ROADMAP.md) first
- Explain the use case and motivation
- Consider backward compatibility
- Discuss performance implications

## 📚 Documentation

### Types of Documentation
- **API docs**: Rustdoc for all public APIs
- **Guides**: High-level usage documentation
- **Examples**: Working code demonstrating features
- **Architecture**: Design decisions and patterns

### Writing Documentation
- Use clear, concise language
- Include working code examples
- Test all code examples in CI
- Link related concepts

## 🔍 Code Review Process

### Before Submitting
- [ ] All tests pass (`just test`)
- [ ] Code is formatted (`just fmt`)
- [ ] No clippy warnings (`just lint`)
- [ ] Documentation updated
- [ ] CHANGELOG.md updated (if applicable)

### Review Criteria
- **Correctness**: Does it work as intended?
- **Performance**: Meets our targets?
- **Compatibility**: Works across feature flags?
- **Documentation**: Clear and complete?
- **Tests**: Adequate coverage?

## 🎉 Recognition

Contributors are recognized in:
- Git commit history
- CHANGELOG.md for significant contributions
- README.md for major features
- Release notes

## 📞 Getting Help

- **GitHub Issues**: For bugs and feature requests
- **GitHub Discussions**: For questions and ideas
- **Documentation**: Check [docs/](./docs/) first

---

*Thank you for contributing to lit-bit! Together we're building a world-class statechart library for Rust.* 🦀❤️ 