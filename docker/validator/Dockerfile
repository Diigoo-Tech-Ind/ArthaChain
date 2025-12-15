# ArthaChain Smart Node (Validator/Miner/Shard/Auto)
# =============================================================================
# FAST BUILD: Uses pre-built binaries. Run ./scripts/build_binaries.sh first.
# Build time: ~30 seconds (vs 15+ minutes with source compilation)
# =============================================================================

# ----------------- RUNTIME STAGE (No compilation!) -----------------
FROM debian:bookworm-slim

# Install runtime dependencies only
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 curl iproute2 \
    && rm -rf /var/lib/apt/lists/*

# Create user and directories
RUN groupadd -r arthachain && useradd -r -g arthachain arthachain
RUN mkdir -p /data /config && chown -R arthachain:arthachain /data /config

# Copy pre-built binaries from local build
COPY target/release/arthachain_node /usr/local/bin/
COPY target/release/ai-jobd /usr/local/bin/

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
