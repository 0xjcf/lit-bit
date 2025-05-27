# Rust Agent project tasks
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

# Target specific package for development runs if needed
dev:
  @echo "Running lit-bit-core (main library) for development..."
  @cargo run -p lit-bit-core

# Top-level test runs all workspace tests
test: test-core test-macro
  @echo "All workspace tests completed."

# Test the core library
test-core:
  @echo "🧪 Testing core library (lit-bit-core)..."
  @cargo nextest run -p lit-bit-core --features std

# Test the procedural macro crate
test-macro:
  @echo "🔬 Testing procedural macro crate (lit-bit-macro)..."
  @cargo test -p lit-bit-macro

test-summary:
  @cargo test 2>&1 | grep "test result"

# --- Lint Tasks ---
# Comprehensive lint: includes pedantic warnings, all features, nightly check, AND CI-exact check
# This is the most thorough check and will catch issues before they hit CI
# Usage: just lint [fix] OR just lint <app_name> [fix]
lint app='workspace' fix='':
  ./scripts/lint_app.sh {{app}} {{fix}}

# Nightly-specific lint with all features (more permissive than CI)
lint-nightly app='workspace' fix='':
  #!/usr/bin/env bash
  set -e
  echo "🌙 Running nightly clippy checks..."
  if ! rustup toolchain list | grep -q "nightly"; then
    echo "❌ Nightly toolchain not installed. Install with: rustup toolchain install nightly"
    exit 1
  fi
  
  if [[ "{{fix}}" == "fix" ]]; then
    echo "🔧 Fixing nightly clippy issues..."
    cargo +nightly clippy --workspace --all-targets --all-features --fix --allow-dirty --allow-staged -- -D warnings
  else
    echo "🔍 Checking for nightly clippy issues..."
    cargo +nightly clippy --workspace --all-targets --all-features -- -D warnings
  fi
  echo "✅ Nightly clippy check complete."

# CI-exact lint commands - matches exactly what CI runs (without --all-features)
# Use this to catch issues that only appear in the CI environment
# Usage: just lint-ci [stable|nightly]
lint-ci toolchain='stable':
  #!/usr/bin/env bash
  set -e
  echo "🤖 Running CI-exact clippy checks ({{toolchain}})..."
  
  if [[ "{{toolchain}}" == "nightly" ]]; then
    if ! rustup toolchain list | grep -q "nightly"; then
      echo "❌ Nightly toolchain not installed. Install with: rustup toolchain install nightly"
      exit 1
    fi
    echo "🔍 Running nightly clippy (CI-exact)..."
    cargo +nightly clippy --all-targets --workspace -- -D warnings
  else
    echo "🔍 Running stable clippy (CI-exact)..."
    cargo clippy --all-targets --workspace -- -D warnings
  fi
  echo "✅ CI-exact clippy check complete."

# Test feature matrix (matches CI exactly) - excludes embassy feature temporarily
test-features:
  #!/usr/bin/env bash
  set -e
  echo "🧪 Testing feature matrix (lit-bit-core only)..."
  cd lit-bit-core
  if ! command -v cargo-hack &> /dev/null; then
    echo "❌ cargo-hack not installed. Install with: cargo install cargo-hack --locked"
    exit 1
  fi
  cargo hack check --feature-powerset --no-dev-deps --exclude-features embassy
  echo "✅ Feature matrix test complete."

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
  @cargo run -p lit-bit-core --example traffic_light --target riscv32imac-unknown-none-elf --no-default-features --release

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
  @cargo build -p lit-bit-core --example traffic_light_cortex_m --target thumbv7m-none-eabi --no-default-features --release
  @echo "\n📏 Size report for traffic_light_cortex_m:"
  @cargo size -p lit-bit-core --example traffic_light_cortex_m --target thumbv7m-none-eabi --release -- -A

# --- Heap Crash Canary (Optional Embedded Test) ---
# This test builds and (optionally) runs the heap_crash example for riscv32.
# It will crash at runtime if heap allocation is attempted, proving the dummy allocator is active.
# Usage: just heap-crash-test-rv
heap-crash-test-rv:
  @echo "🚨 Building heap_crash example (RISC-V, no_std, dummy allocator)..."
  @cargo build -p lit-bit-core --example heap_crash --target riscv32imac-unknown-none-elf --no-default-features --release
  @echo "If you run this on QEMU or hardware, it should crash if heap allocation is attempted."
  @echo "(Not run by default in CI; for manual/optional validation.)"

heap-crash-test-cm:
  @echo "🚨 Building heap_crash example (Cortex-M, no_std, dummy allocator)..."
  @cargo build -p lit-bit-core --example heap_crash --target thumbv7m-none-eabi --no-default-features --release
  @echo "If you run this on QEMU or hardware, it should crash if heap allocation is attempted."
  @echo "(Not run by default in CI; for manual/optional validation.)"
