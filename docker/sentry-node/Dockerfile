# ArthaChain Sentry Node - Production Ready
# Role: Gateway/Shield node for DDoS protection
# 
# Sentry nodes act as a protective layer between the public internet
# and the internal validator nodes. They:
#   - Accept public P2P connections
#   - Filter malicious traffic
#   - Relay valid messages to validators
#   - Protect validator IP addresses from exposure
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

# Build the sentry node binary with production features
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
RUN groupadd -r -g 1003 arthachain && \
    useradd -r -u 1003 -g arthachain -s /bin/false -d /app arthachain

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

# Environment variables for Sentry Node
ENV RUST_LOG=info
ENV API_PORT=1900
ENV P2P_PORT=8084
ENV METRICS_PORT=9184

# Sentry-specific: Connect to internal validator nodes
# These should be set in docker-compose or K8s config
ENV VALIDATOR_NODES=validator-node:8084

# Expose ports
# 1900 - ArthaChain API (public facing)
# 8084 - P2P network (public facing)
# 9184 - Metrics (internal)
EXPOSE 1900 8084 9184

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=45s --retries=3 \
    CMD curl -f http://localhost:1900/health || exit 1

# Volume for persistent data
VOLUME ["/app/data", "/app/config"]

# Entry point with sentry configuration
ENTRYPOINT ["arthachain_node"]
CMD ["--api-port", "1900", "--p2p-port", "8084", "--metrics-port", "9184"]
