# ArthaChain RPC Node
# =============================================================================
# FAST BUILD: Uses pre-built binaries. Run ./scripts/build_binaries.sh first.
# Build time: ~30 seconds
# =============================================================================

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 curl \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -r arthachain && useradd -r -g arthachain arthachain
RUN mkdir -p /data /config && chown -R arthachain:arthachain /data /config

# Copy pre-built binary
COPY target/release/arthachain_node /usr/local/bin/

WORKDIR /data
USER arthachain

ENV NODE_ROLE=rpc
ENV RUST_LOG=info
ENV DATA_DIR=/data

EXPOSE 8545 9944

ENTRYPOINT ["arthachain_node"]
CMD ["--rpc-external", "--rpc-cors", "all", "--base-path", "/data"]
