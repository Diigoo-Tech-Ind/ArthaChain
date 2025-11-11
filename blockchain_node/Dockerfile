# Multi-stage Dockerfile for ArthaChain
FROM rust:1.82-slim as builder

WORKDIR /app

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./
COPY blockchain_node/Cargo.toml ./blockchain_node/

# Copy source code
COPY blockchain_node ./blockchain_node

# Build dependencies first (caching layer)
RUN cargo build --release --features production --bin arthachain_node || true

# Build the application
RUN cargo build --release --features production --bin arthachain_node

# Runtime stage
FROM debian:bookworm-slim as runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    libgcc-s1 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false arthachain

# Copy binary from builder stage
COPY --from=builder /app/target/release/arthachain_node /usr/local/bin/arthachain_node

# Set ownership and permissions
RUN chown arthachain:arthachain /usr/local/bin/arthachain_node && \
    chmod +x /usr/local/bin/arthachain_node

USER arthachain

EXPOSE 8080 9944 8545

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["arthachain_node"]
