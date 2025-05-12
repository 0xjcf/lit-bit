# Rust Agent project tasks
default:
  @just --list

dev:
  @cargo run

test:
  @cargo nextest run # Use nextest if available

lint:
  @cargo clippy --all-targets -- -D warnings

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
