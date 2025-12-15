# ArthaChain AI Worker
# =============================================================================
# FAST BUILD: Uses pre-built binaries. Run ./scripts/build_binaries.sh first.
# Build time: ~30 seconds
# =============================================================================

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 curl python3 \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -r arthachain && useradd -r -g arthachain arthachain
RUN mkdir -p /data /config && chown -R arthachain:arthachain /data /config

# Copy pre-built binary
COPY target/release/ai-jobd /usr/local/bin/

WORKDIR /data
USER arthachain

ENV NODE_ROLE=ai-worker
ENV RUST_LOG=info

EXPOSE 8084

ENTRYPOINT ["ai-jobd"]
CMD []
