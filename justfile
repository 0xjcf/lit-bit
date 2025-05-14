# Rust Agent project tasks
default:
  @just --list

dev:
  @cargo run

test:
  @cargo nextest run --lib # Run only library tests

lint:
  @cargo clippy --all-targets --all-features -- -W clippy::pedantic -D warnings

build:
  @cargo build --release

# --- Docker Commands ---
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

# Run the traffic_light example on RISC-V QEMU
run-rv: kill-qemu
  @echo "Running traffic_light example on RISC-V QEMU..."
  @cargo run --example traffic_light --target riscv32imac-unknown-none-elf

# --- Code Quality & Analysis ---
coverage:
  @echo "Generating HTML coverage report..."
  @cargo llvm-cov
  @echo "HTML report generated in target/llvm-cov-html/index.html"
  @printf '\nText summary for AI context:\n'
  @cargo llvm-cov report --text --output-dir target/llvm-cov

# Consider adding a clean-coverage task if needed
# clean-coverage:
#   @echo "Cleaning coverage artifacts..."
#   @cargo llvm-cov clean
#   @rm -rf target/llvm-cov-html

size-check-cortex-m:
  @echo "🔍 Building and checking firmware size for Cortex-M example..."
  @cargo build --example traffic_light_cortex_m --target thumbv7m-none-eabi --release
  @echo "\nSize report for traffic_light_cortex_m:"
  @cargo size --example traffic_light_cortex_m --target thumbv7m-none-eabi --release -- -A # -A for detailed archive view, or remove for summary
