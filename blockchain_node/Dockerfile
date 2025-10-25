# Multi-stage Dockerfile for ArthaChain
FROM rust:1.75-slim as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY blockchain_node ./blockchain_node
COPY src ./src

# Build the application
RUN cargo build --release --features production

# Runtime stage
FROM debian:bookworm-slim as runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false arthachain

# Copy binary from builder stage
COPY --from=builder /app/target/release/blockchain_node /usr/local/bin/

# Set ownership and permissions
RUN chown arthachain:arthachain /usr/local/bin/blockchain_node
RUN chmod +x /usr/local/bin/blockchain_node

USER arthachain

EXPOSE 8080 9944

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["blockchain_node"]
