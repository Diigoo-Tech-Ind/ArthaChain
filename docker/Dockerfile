# ============================================================================
# ArthaChain Universal Node - One Image for All Roles
# Supports: Validator, Miner, RPC, AI Worker, P2P Relay, Storage Node
# ============================================================================

# ----------------- BUILDER STAGE -----------------
FROM rust:1.91-slim as builder

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
    python3 \
    python3-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy entire workspace (simplifies build context)
COPY . .

# Build all binaries in the workspace (arthachain_node + ai-jobd)
# We enable "production" and "evm-runtime" features for maximum capability
ENV CARGO_BUILD_JOBS=4
RUN cargo build --release --workspace --features "production,evm-runtime"

# Strip binaries to reduce image size
RUN strip target/release/arthachain_node || true
RUN strip target/release/ai-jobd || true

# ----------------- RUNTIME STAGE -----------------
FROM debian:bookworm-slim as runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    iproute2 \
    && rm -rf /var/lib/apt/lists/*

# Create dedicated user
RUN groupadd -r arthachain && useradd -r -g arthachain arthachain

# Create standard directories
# /data - Blockchain data
# /config - Configuration files
RUN mkdir -p /data /config && \
    chown -R arthachain:arthachain /data /config

# Copy binaries
COPY --from=builder /app/target/release/arthachain_node /usr/local/bin/
COPY --from=builder /app/target/release/ai-jobd /usr/local/bin/
COPY --from=builder /app/target/release/api_server /usr/local/bin/
COPY --from=builder /app/target/release/real_time_api_server /usr/local/bin/

# Copy entrypoint script
COPY docker/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

# Set working directory
WORKDIR /data

# Default environment variables
ENV NODE_ROLE=full
ENV RUST_LOG=info
ENV DATA_DIR=/data
ENV API_PORT=8080
ENV P2P_PORT=8084

# Expose all standard ports
# 8080: API (HTTP)
# 8545: API (EVM RPC)
# 9944: API (gRPC)
# 8084: P2P (Consensus)
# 30303: P2P (EVM)
# 9184: Metrics
# 8081: AI Job Daemon
EXPOSE 8080 8545 9944 8084 30303 9184 8081

# Switch to user
USER arthachain

# Use our smart entrypoint
ENTRYPOINT ["entrypoint.sh"]
CMD []
