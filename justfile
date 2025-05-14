# Rust Agent project tasks
default:
  @just --list

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
  @cargo nextest run -p lit-bit-core

# Test the procedural macro crate
test-macro:
  @echo "🔬 Testing procedural macro crate (lit-bit-macro)..."
  # Ensure you are in the workspace root or cd to lit-bit-macro if needed
  # cargo test -p lit-bit-macro should work from workspace root.
  @cargo test -p lit-bit-macro

# Lint all workspace members
lint:
  @echo "Linting workspace..."
  @cargo clippy --workspace --all-targets --all-features -- -W clippy::pedantic -D warnings

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
  @echo "Running traffic_light example (from lit-bit-core) on RISC-V QEMU..."
  @cargo run -p lit-bit-core --example traffic_light --target riscv32imac-unknown-none-elf

# --- Code Quality & Analysis ---
coverage: # Coverage might also need --workspace or to target specific packages
  @echo "Generating HTML coverage report (for workspace)..."
  @cargo llvm-cov --workspace # Or specify packages if needed and adjust report paths
  @echo "HTML report generated in target/llvm-cov/html/index.html" # Path might change with workspace
  @printf '\nText summary for AI context:\n'
  @cargo llvm-cov report --text --output-dir target/llvm-cov # Path might change

size-check-cortex-m:
  @echo "🔍 Building and checking firmware size for Cortex-M example (from lit-bit-core)..."
  @cargo build -p lit-bit-core --example traffic_light_cortex_m --target thumbv7m-none-eabi --release
  @echo "\nSize report for traffic_light_cortex_m:"
  @cargo size -p lit-bit-core --example traffic_light_cortex_m --target thumbv7m-none-eabi --release -- -A
