# Multi-stage Dockerfile for ArthaChain
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY blockchain_node ./blockchain_node
COPY src ./src

# Build the application
RUN cargo build --release --features full

# Runtime stage
FROM debian:bookworm-slim as runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false arthachain

# Copy binary from builder stage
COPY --from=builder /app/target/release/arthachain /usr/local/bin/arthachain

# Set ownership and permissions
RUN chown arthachain:arthachain /usr/local/bin/arthachain
RUN chmod +x /usr/local/bin/arthachain

USER arthachain

EXPOSE 8080 9944

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["arthachain"]
