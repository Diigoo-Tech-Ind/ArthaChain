# ArthaChain API Gateway - Production Ready
# Role: REST API endpoint for blockchain interactions
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

# Build both API server binaries
RUN cargo build --release --bin api_server --bin real_time_api_server --features production

# Strip binaries to reduce size
RUN strip /app/target/release/api_server || true
RUN strip /app/target/release/real_time_api_server || true

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
RUN groupadd -r -g 1004 arthachain && \
    useradd -r -u 1004 -g arthachain -s /bin/false -d /app arthachain

# Create directories
RUN mkdir -p /app/data /app/config /app/logs && \
    chown -R arthachain:arthachain /app

# Copy binaries from builder stage
COPY --from=builder --chown=arthachain:arthachain /app/target/release/api_server /usr/local/bin/api_server
COPY --from=builder --chown=arthachain:arthachain /app/target/release/real_time_api_server /usr/local/bin/real_time_api_server

# Set executable permissions
RUN chmod +x /usr/local/bin/api_server /usr/local/bin/real_time_api_server

# Switch to non-root user
USER arthachain

WORKDIR /app

# Environment variables for API gateway
ENV RUST_LOG=info
ENV NODE_ROLE=api-gateway
ENV API_PORT=1930
ENV NETWORK=mainnet
# Backend node URL for proxying blockchain requests
ENV BACKEND_NODE_URL=http://validator-node:1900

# Expose ports
# 1930 - REST API (ArthaChain Explorer API port)
# 3000 - Alternative API port
EXPOSE 1930 3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:1930/health || exit 1

# Volume for configuration
VOLUME ["/app/config"]

# Entry point
ENTRYPOINT ["api_server"]
CMD ["--port", "1930", "--network", "mainnet"]
