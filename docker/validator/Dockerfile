# ----------------- BUILDER STAGE -----------------
FROM rust:1.91-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates build-essential cmake clang libclang-dev protobuf-compiler \
    python3 python3-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# Build specific binary
RUN cargo build --release --bin arthachain_node --features "production"
RUN strip target/release/arthachain_node

# ----------------- RUNTIME STAGE -----------------
FROM debian:bookworm-slim

# Install runtime dependencies only
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 curl iproute2 \
    && rm -rf /var/lib/apt/lists/*

# Create user and directories
RUN groupadd -r arthachain && useradd -r -g arthachain arthachain
RUN mkdir -p /data /config && chown -R arthachain:arthachain /data /config

# Copy built binary from builder stage
COPY --from=builder /app/target/release/arthachain_node /usr/local/bin/

# Copy Smart Entrypoint
COPY docker/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

WORKDIR /data
USER arthachain

# Smart Node Configuration
ENV NODE_ROLE=auto
ENV RUST_LOG=info
ENV DATA_DIR=/data
ENV ENABLE_GPU=auto
ENV ENABLE_MINING=true
ENV ENABLE_SHARDING=true

EXPOSE 8080 8545 9944 8084 9184 8081

ENTRYPOINT ["entrypoint.sh"]
CMD []
