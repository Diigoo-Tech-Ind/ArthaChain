# ArthaChain Storage Node - Production Ready
# Role: Data persistence, blockchain storage, SVDB integration
# Updated: 2025

# ============== BUILD STAGE ==============
FROM rust:1.83-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    ca-certificates \
    build-essential \
    cmake \
    clang \
    libclang-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY blockchain_node ./blockchain_node
COPY src ./src

# Build dependencies first (caching layer)
RUN cargo fetch

# Build the storage node binary with full features
RUN cargo build --release --bin arthachain_node --features production

# Strip binary to reduce size
RUN strip /app/target/release/arthachain_node || true

# ============== RUNTIME STAGE ==============
FROM debian:bookworm-slim as runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    libgcc-s1 \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* \
    && rm -rf /tmp/* \
    && rm -rf /var/tmp/*

# Create non-root user
RUN groupadd -r -g 1007 arthachain && \
    useradd -r -u 1007 -g arthachain -s /bin/false -d /app arthachain

# Create directories for storage
RUN mkdir -p /app/data/blockchain \
    /app/data/rocksdb \
    /app/data/svdb \
    /app/data/memmap \
    /app/config \
    /app/logs && \
    chown -R arthachain:arthachain /app

# Copy binary from builder stage
COPY --from=builder --chown=arthachain:arthachain /app/target/release/arthachain_node /usr/local/bin/arthachain_node

# Set executable permissions
RUN chmod +x /usr/local/bin/arthachain_node

# Switch to non-root user
USER arthachain

WORKDIR /app

# Environment variables for storage node
ENV RUST_LOG=info
ENV NODE_ROLE=storage
ENV ENABLE_MINING=false
ENV ENABLE_CONSENSUS=false
ENV ENABLE_STORAGE=true
ENV ENABLE_SVDB=true
ENV ENABLE_ROCKSDB=true
ENV ENABLE_MEMMAP=true
ENV API_PORT=1900
ENV P2P_PORT=8084
ENV METRICS_PORT=9184
# Storage configuration
ENV STORAGE_PATH=/app/data
ENV ROCKSDB_PATH=/app/data/rocksdb
ENV SVDB_PATH=/app/data/svdb
ENV MAX_STORAGE_SIZE_GB=500
ENV ENABLE_COMPRESSION=true

# Expose ports
# 1900 - ArthaChain API (storage queries)
# 8084 - P2P network (block sync)
# 9184 - Metrics
EXPOSE 1900 8084 9184

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD curl -f http://localhost:1900/health || exit 1

# Volume for persistent data - CRITICAL for storage node
VOLUME ["/app/data", "/app/config"]

# Entry point with storage configuration
ENTRYPOINT ["arthachain_node"]
CMD ["--api-port", "1900", "--p2p-port", "8084", "--metrics-port", "9184"]
