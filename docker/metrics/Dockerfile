# ArthaChain Metrics Exporter
# =============================================================================
# FAST BUILD: Uses pre-built binaries. Run ./scripts/build_binaries.sh first.
# =============================================================================

# ----------------- BUILDER STAGE -----------------
FROM rust:1.91-slim as builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates build-essential cmake clang libclang-dev protobuf-compiler \
    python3 python3-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN cargo build --release --bin arthachain_node --features "production"
RUN strip target/release/arthachain_node

# ----------------- RUNTIME STAGE -----------------
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 curl \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -r arthachain && useradd -r -g arthachain arthachain
RUN mkdir -p /data /config && chown -R arthachain:arthachain /data /config

COPY --from=builder /app/target/release/arthachain_node /usr/local/bin/

WORKDIR /data
USER arthachain

ENV NODE_ROLE=metrics
ENV RUST_LOG=info

EXPOSE 9184

ENTRYPOINT ["arthachain_node"]
CMD ["--prometheus-external", "--prometheus-port", "9184"]
