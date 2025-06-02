# Lit-bit workspace automation tasks
default:
  @just --list

# --- New xtask-based commands ---
# Run CI checks for all targets
ci:
  @cargo run --manifest-path xtask/Cargo.toml -- check-all

# Run CI for a specific target
ci-target target:
  @cargo run --manifest-path xtask/Cargo.toml -- ci --target {{target}}

# Run all tests via xtask
test-all:
  @cargo run --manifest-path xtask/Cargo.toml -- test

# Run benchmarks in smoke mode
bench-smoke:
  @cargo run --manifest-path xtask/Cargo.toml -- bench --smoke

# Run full benchmarks
bench:
  @cargo run --manifest-path xtask/Cargo.toml -- bench

# Check that benchmarks compile
bench-check:
  @echo "🔍 Checking benchmark compilation..."
  @cargo check -p lit-bit-bench

# Target specific package for development runs if needed
dev:
  @echo "Running lit-bit-core (main library) for development..."
  @cargo run -p lit-bit-core

# Top-level test runs all workspace tests
test: test-core test-macro test-integration
  @echo "All workspace tests completed."

# Test the core library (unit tests only)
test-core:
  @echo "🧪 Testing core library (lit-bit-core)..."
  @cargo test -p lit-bit-core --lib --bins

# Test the procedural macro crate
test-macro:
  @echo "🔬 Testing procedural macro crate (lit-bit-macro)..."
  @cargo test -p lit-bit-macro

# Test the integration test suite
test-integration:
  @echo "🔬 Testing integration test suite (lit-bit-tests)..."
  @cargo test -p lit-bit-tests

# Use xtask for comprehensive testing: just test-all
test-summary:
  @cargo test 2>&1 | grep "test result"

# --- Lint Tasks ---
# Professional-grade comprehensive linting with feature matrix testing
# Tests all valid feature combinations without triggering mutually exclusive conflicts
# Usage: just lint
lint:
  ./scripts/lint.sh

# Quick lint check for development (core library only, fastest feedback)
lint-quick:
  #!/usr/bin/env bash
  set -e
  echo "🔍 Quick lint check (core library only)..."
  cargo clippy -p lit-bit-core --lib --features std -- -D warnings
  echo "✅ Quick lint complete."

# Nightly-specific lint with individual feature testing (more permissive than CI)
lint-nightly:
  #!/usr/bin/env bash
  set -e
  echo "🌙 Running nightly clippy checks with feature matrix..."
  if ! rustup toolchain list | grep -q "nightly"; then
    echo "❌ Nightly toolchain not installed. Install with: rustup toolchain install nightly"
    exit 1
  fi
  
  echo "🔍 Testing core with nightly clippy..."
  cargo +nightly clippy -p lit-bit-core --lib --features std -- -D warnings
  echo "✅ Nightly clippy check complete."

# Beta-specific lint to catch issues early (matches CI beta job)
lint-beta:
  #!/usr/bin/env bash
  set -e
  echo "🧪 Running beta clippy checks (matches CI matrix)..."
  if ! rustup toolchain list | grep -q "beta"; then
    echo "❌ Beta toolchain not installed. Install with: rustup toolchain install beta"
    exit 1
  fi
  
  echo "🔍 Testing workspace with beta clippy..."
  cargo +beta clippy --all-targets --workspace -- -D warnings
  echo "✅ Beta clippy check complete."

# CI-matching lint with all toolchains (stable, beta, nightly)
lint-ci:
  #!/usr/bin/env bash
  set -e
  echo "🚀 Running CI-matching lint checks with all toolchains..."
  
  echo "📦 Testing with stable toolchain..."
  ./scripts/lint.sh
  
  if rustup toolchain list | grep -q "beta"; then
    echo "📦 Testing with beta toolchain..."
    cargo +beta clippy --all-targets --workspace -- -D warnings
  else
    echo "⚠️  Beta toolchain not available, install with: rustup toolchain install beta"
  fi
  
  if rustup toolchain list | grep -q "nightly"; then
    echo "📦 Testing with nightly toolchain..."
    cargo +nightly clippy --all-targets --workspace -- -D warnings
  else
    echo "⚠️  Nightly toolchain not available, install with: rustup toolchain install nightly"
  fi
  
  echo "✅ All CI-matching lint checks complete!"

# Format check and fix
fmt:
  @echo "🎨 Formatting code..."
  @cargo fmt --all

fmt-check:
  @echo "🎨 Checking code formatting..."
  @cargo fmt --all --check

# Build all workspace members for release
build:
  @echo "Building workspace for release..."
  @cargo build --workspace --release

# --- Docker Commands --- (These might not need changes if they operate on the whole repo context)
docker-build:
  @docker build -t lit-bit -f docker/Dockerfile .

docker-dev:
  @docker compose -f docker/docker-compose.yml up --build -d
  @echo "Container started in detached mode. Following logs (Ctrl+C to stop logs)..."
  @docker compose -f docker/docker-compose.yml logs --follow dev

docker-test:
  @echo "Docker tests not typically run for agents this way."

docker-stop:
  @docker compose -f docker/docker-compose.yml down

# --- RISC-V QEMU Tasks ---
kill-qemu:
  @echo "Attempting to kill existing QEMU processes..."
  @pkill -f qemu-system-riscv32 || echo "No QEMU processes found or pkill not available/effective."

# Run the traffic_light example from lit-bit-core on RISC-V QEMU
run-rv: kill-qemu
  @echo "🚀 Running traffic_light example (RISC-V QEMU, no_std)..."
  @cargo run -p lit-bit-core --example traffic_light --target riscv32imac-unknown-none-elf --no-default-features --features panic-halt --release

# --- Code Quality & Analysis ---
coverage: # Coverage might also need --workspace or to target specific packages
  @echo "Generating HTML coverage report (for workspace)..."
  @cargo llvm-cov --workspace # Or specify packages if needed and adjust report paths
  @echo "HTML report generated in target/llvm-cov/html/index.html" # Path might change with workspace
  @printf '\nText summary for AI context:\n'
  @cargo llvm-cov report --text --output-dir target/llvm-cov # Path might change

# Cortex-M size check with strict no_std build
size-check-cortex-m:
  @echo "🔍 Building traffic_light_cortex_m (no_std)..."
  @cargo build -p lit-bit-core --example traffic_light_cortex_m --target thumbv7m-none-eabi --no-default-features --features panic-halt --release
  @echo "\n📏 Size report for traffic_light_cortex_m:"
  @cargo size -p lit-bit-core --example traffic_light_cortex_m --target thumbv7m-none-eabi --no-default-features --features panic-halt --release -- -A

# --- Heap Crash Canary (Optional Embedded Test) ---
# This test builds and (optionally) runs the heap_crash example for riscv32.
# It will crash at runtime if heap allocation is attempted, proving the dummy allocator is active.
# Usage: just heap-crash-test-rv
heap-crash-test-rv:
  @echo "🚨 Building heap_crash example (RISC-V, no_std, dummy allocator)..."
  @cargo build -p lit-bit-core --example heap_crash --target riscv32imac-unknown-none-elf --no-default-features --features panic-halt --release
  @echo "If you run this on QEMU or hardware, it should crash if heap allocation is attempted."
  @echo "(Not run by default in CI; for manual/optional validation.)"

heap-crash-test-cm:
  @echo "🚨 Building heap_crash example (Cortex-M, no_std, dummy allocator)..."
  @cargo build -p lit-bit-core --example heap_crash --target thumbv7m-none-eabi --no-default-features --features panic-halt --release
  @echo "If you run this on QEMU or hardware, it should crash if heap allocation is attempted."
  @echo "(Not run by default in CI; for manual/optional validation.)"
