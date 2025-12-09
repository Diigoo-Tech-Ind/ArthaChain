# ArthaChain RPC Node - Production Ready
# Role: Read-only JSON-RPC service for EVM compatibility
#
# RPC nodes provide:
#   - Ethereum-compatible JSON-RPC API (eth_*, net_*, web3_*)
#   - Read-only access to blockchain state
#   - Query capabilities without participating in consensus
#   - High availability endpoints for dApps and wallets
#
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

# Build the node binary with EVM runtime features
RUN cargo build --release --bin arthachain_node --features "evm-runtime,production"

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
RUN groupadd -r -g 1002 arthachain && \
    useradd -r -u 1002 -g arthachain -s /bin/false -d /app arthachain

# Create directories
RUN mkdir -p /app/data /app/config /app/logs && \
    chown -R arthachain:arthachain /app

# Copy binary from builder stage
COPY --from=builder --chown=arthachain:arthachain /app/target/release/arthachain_node /usr/local/bin/arthachain_node

# Set executable permissions
RUN chmod +x /usr/local/bin/arthachain_node

# Switch to non-root user
USER arthachain

WORKDIR /app

# Environment variables for RPC Node
ENV RUST_LOG=info
ENV API_PORT=1900
ENV P2P_PORT=8084
ENV METRICS_PORT=9184

# RPC Node syncs from other nodes
ENV SYNC_FROM_PEERS=true

# Expose ports
# 8545 - EVM JSON-RPC (Ethereum compatible)
# 1900 - ArthaChain API
# 8084 - P2P network (for syncing)
# 9184 - Metrics
EXPOSE 8545 1900 8084 9184

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=45s --retries=3 \
    CMD curl -f http://localhost:8545/ -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' || curl -f http://localhost:1900/health || exit 1

# Volume for persistent data
VOLUME ["/app/data", "/app/config"]

# Entry point with RPC configuration
ENTRYPOINT ["arthachain_node"]
CMD ["--api-port", "1900", "--p2p-port", "8084", "--metrics-port", "9184"]
