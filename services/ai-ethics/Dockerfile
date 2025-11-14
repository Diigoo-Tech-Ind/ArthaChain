# AI Ethics Service
FROM rust:1.82-slim as builder

WORKDIR /app

# Copy workspace root files
COPY Cargo.toml Cargo.lock ./

# Copy service-specific Cargo.toml
COPY services/ai-ethics/Cargo.toml ./services/ai-ethics/

# Copy source code
COPY services/ai-ethics ./services/ai-ethics

# Build dependencies first (caching layer)
RUN cargo build --release --bin ai-ethics || true

# Build the application
RUN cargo build --release --bin ai-ethics

# Runtime stage
FROM debian:bookworm-slim as runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    libgcc-s1 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false ai-ethics

# Copy binary from builder stage
COPY --from=builder /app/target/release/ai-ethics /usr/local/bin/ai-ethics

# Set ownership and permissions
RUN chown ai-ethics:ai-ethics /usr/local/bin/ai-ethics && \
    chmod +x /usr/local/bin/ai-ethics

USER ai-ethics

EXPOSE 8089

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8089/health || exit 1

CMD ["ai-ethics"]

