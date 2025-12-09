# ArthaChain gRPC Server - Production Ready
# Role: gRPC API endpoint for high-performance RPC
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
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY blockchain_node ./blockchain_node
COPY src ./src

# Build dependencies first (caching layer)
RUN cargo fetch

# Build the gRPC server binary
RUN cargo build --release --bin grpc_server --features production

# Strip binary to reduce size
RUN strip /app/target/release/grpc_server || true

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
RUN groupadd -r -g 1008 arthachain && \
    useradd -r -u 1008 -g arthachain -s /bin/false -d /app arthachain

# Create directories
RUN mkdir -p /app/data /app/config /app/logs && \
    chown -R arthachain:arthachain /app

# Copy binary from builder stage
COPY --from=builder --chown=arthachain:arthachain /app/target/release/grpc_server /usr/local/bin/grpc_server

# Set executable permissions
RUN chmod +x /usr/local/bin/grpc_server

# Switch to non-root user
USER arthachain

WORKDIR /app

# Environment variables for gRPC server
ENV RUST_LOG=info
ENV NODE_ROLE=grpc-server
ENV GRPC_PORT=9944
# Backend node for state queries
ENV BACKEND_NODE_URL=http://validator-node:1900

# Expose ports
# 9944 - gRPC API
EXPOSE 9944

# Health check using gRPC health check or fallback to TCP check
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD curl -sf http://localhost:9944/ || nc -z localhost 9944 || exit 1

# Volume for configuration
VOLUME ["/app/config"]

# Entry point
ENTRYPOINT ["grpc_server"]
CMD ["--port", "9944"]
