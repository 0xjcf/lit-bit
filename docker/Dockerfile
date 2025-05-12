# Use the official Rust image. Use rust:1-slim for latest stable 1.x release.
# Consider using -slim variants for smaller image sizes.
FROM rust:1-slim AS chef
WORKDIR /app
# Install system dependencies needed for building some crates (like openssl-sys)
RUN apt-get update && \
    apt-get install -y --no-install-recommends pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*
# Install cargo-chef
RUN cargo install cargo-chef

FROM chef AS planner
# Copy project manifests
COPY Cargo.toml Cargo.lock ./ 
# Copy source code (needed for chef to determine dependencies)
COPY src ./src
# Compute dependencies recipe
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
WORKDIR /app
# Copy the dependencies recipe
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this layer is cached as long as the recipe doesn't change
RUN cargo chef cook --release --recipe-path recipe.json

# Copy application code
COPY Cargo.toml Cargo.lock ./ 
COPY src ./src
# Build the application
RUN cargo build --release --bin lit-bit # Assumes binary name matches project name

# --- Test Stage ---
# Use the builder stage as a base, which has the toolchain and dependencies
FROM builder AS test
WORKDIR /app
# Install cargo-nextest
RUN cargo install cargo-nextest
# Copy source code again in case it changed, or rely on builder's copy
# COPY . .
# Set the default command to run tests
CMD ["cargo", "nextest", "run"]

# --- Runtime Stage ---
# Use a minimal base image for the final stage
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/lit-bit /usr/local/bin/

# Copy any necessary runtime assets (e.g., .env files if not mounted)
# COPY .env.production .

# Set necessary environment variables
ENV RUST_LOG=info
ENV PORT=3000
# Ensure the binary can find shared libraries if needed
# ENV LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH

# Expose the application port defined by the environment variable
EXPOSE ${port}

# Command to run the application
ENTRYPOINT ["/usr/local/bin/lit-bit"] 