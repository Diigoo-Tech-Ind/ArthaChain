# syntax=docker/dockerfile:1
# Optimized Dockerfile for ArthaChain Node
# Uses cargo-chef for dependency caching and multi-stage build

# Base stage with cargo-chef
FROM rust:1.75-slim-bookworm AS chef
WORKDIR /app
RUN cargo install cargo-chef --locked

# Planner stage - computes recipe file
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Builder stage - builds dependencies then application
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is cached!
RUN cargo chef cook --release --recipe-path recipe.json

# Build application
COPY . .
RUN cargo build --release --bin production_node --bin simple_api_server

# Runtime stage
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Install minimal runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r arthachain && useradd -r -g arthachain arthachain

# Copy binaries
COPY --from=builder /app/target/release/production_node /usr/local/bin/arthachain_node
COPY --from=builder /app/target/release/simple_api_server /usr/local/bin/api_server

# Set ownership
RUN chown -R arthachain:arthachain /app /usr/local/bin/arthachain_node /usr/local/bin/api_server

# Switch to user
USER arthachain

# Expose ports
EXPOSE 8080 8545 30303 9944

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8545/health || exit 1

# Default command
CMD ["arthachain_node"]
