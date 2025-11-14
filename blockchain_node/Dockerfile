# Multi-stage Dockerfile for ArthaChain - Production Ready
# Updated: 2025 - Latest stable versions with security best practices

# Build stage
FROM rust:1.83-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy dependency manifests first (better caching)
COPY Cargo.toml Cargo.lock ./
COPY blockchain_node/Cargo.toml ./blockchain_node/

# Pre-build dependencies (caching layer)
RUN cargo fetch

# Build dependencies only (caching optimization)
RUN cargo build --release --features production --target-dir /app/target || true

# Copy source code
COPY blockchain_node ./blockchain_node

# Build all required binaries
RUN cargo build --release --features production --bin arthachain_node && \
    cargo build --release --features production --bin arthachain_global_node && \
    cargo build --release --features production --bin real_time_api_server && \
    cargo build --release --features production --bin api_server

# Strip binaries to reduce size
RUN strip /app/target/release/arthachain_node && \
    strip /app/target/release/arthachain_global_node && \
    strip /app/target/release/real_time_api_server && \
    strip /app/target/release/api_server

# Runtime stage - minimal base image
FROM debian:bookworm-slim as runtime

# Install only runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    libgcc-s1 \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* \
    && rm -rf /tmp/* \
    && rm -rf /var/tmp/*

# Create non-root user with specific UID/GID for security
RUN groupadd -r -g 1000 arthachain && \
    useradd -r -u 1000 -g arthachain -s /bin/false -d /nonexistent arthachain

# Create necessary directories with proper permissions
RUN mkdir -p /arthachain/data /arthachain/logs /arthachain/config && \
    chown -R arthachain:arthachain /arthachain

# Copy all binaries from builder stage
COPY --from=builder --chown=arthachain:arthachain /app/target/release/arthachain_node /usr/local/bin/arthachain_node
COPY --from=builder --chown=arthachain:arthachain /app/target/release/arthachain_global_node /usr/local/bin/arthachain_global_node
COPY --from=builder --chown=arthachain:arthachain /app/target/release/real_time_api_server /usr/local/bin/real_time_api_server
COPY --from=builder --chown=arthachain:arthachain /app/target/release/api_server /usr/local/bin/api_server

# Set executable permissions
RUN chmod +x /usr/local/bin/arthachain_node && \
    chmod +x /usr/local/bin/arthachain_global_node && \
    chmod +x /usr/local/bin/real_time_api_server && \
    chmod +x /usr/local/bin/api_server

# Switch to non-root user
USER arthachain

# Set working directory
WORKDIR /arthachain

# Expose ports
EXPOSE 8080 9944 8545

# Health check with proper intervals
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Use exec form for proper signal handling
CMD ["arthachain_node"]
