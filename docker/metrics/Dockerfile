# ArthaChain Metrics Exporter
# =============================================================================
# FAST BUILD: Uses pre-built binaries. Run ./scripts/build_binaries.sh first.
# =============================================================================

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 curl \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -r arthachain && useradd -r -g arthachain arthachain
RUN mkdir -p /data /config && chown -R arthachain:arthachain /data /config

COPY target/release/arthachain_node /usr/local/bin/

WORKDIR /data
USER arthachain

ENV NODE_ROLE=metrics
ENV RUST_LOG=info

EXPOSE 9184

ENTRYPOINT ["arthachain_node"]
CMD ["--prometheus-external", "--prometheus-port", "9184"]
