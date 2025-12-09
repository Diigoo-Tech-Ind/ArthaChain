# ArthaChain WebSocket RPC - Production Ready
# Role: Real-time event subscriptions via WebSocket
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

# Build the node binary with WebSocket features
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
RUN groupadd -r -g 1005 arthachain && \
    useradd -r -u 1005 -g arthachain -s /bin/false -d /app arthachain

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

# Environment variables for WebSocket RPC
ENV RUST_LOG=info
ENV NODE_ROLE=websocket-rpc
ENV ENABLE_MINING=false
ENV ENABLE_CONSENSUS=false
ENV ENABLE_WEBSOCKET=true
ENV API_PORT=1900
ENV WEBSOCKET_PORT=8546
ENV P2P_PORT=8084
ENV METRICS_PORT=9184
# Backend node for state synchronization
ENV BACKEND_NODE_URL=http://validator-node:1900

# Expose ports
# 8546 - WebSocket RPC
# 1900 - ArthaChain API
# 8084 - P2P network
# 9184 - Metrics
EXPOSE 8546 1900 8084 9184

# Health check - check main API since WebSocket health check is more complex
HEALTHCHECK --interval=30s --timeout=10s --start-period=45s --retries=3 \
    CMD curl -f http://localhost:1900/health || exit 1

# Volume for persistent data
VOLUME ["/app/data", "/app/config"]

# Entry point with WebSocket configuration
ENTRYPOINT ["arthachain_node"]
CMD ["--api-port", "1900", "--p2p-port", "8084", "--metrics-port", "9184"]
