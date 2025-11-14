# AI Runtime Service - Handles Docker container orchestration for AI jobs
FROM rust:1.82-slim as builder

WORKDIR /app

# Copy workspace root files
COPY Cargo.toml Cargo.lock ./

# Copy service-specific Cargo.toml
COPY services/ai-runtime/Cargo.toml ./services/ai-runtime/

# Copy source code
COPY services/ai-runtime ./services/ai-runtime

# Build dependencies first (caching layer)
RUN cargo build --release --bin ai-runtime || true

# Build the application
RUN cargo build --release --bin ai-runtime

# Runtime stage
FROM debian:bookworm-slim as runtime

# Install runtime dependencies
# Note: Docker socket should be mounted from host, not docker.io package
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    libgcc-s1 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false ai-runtime

# Copy binary from builder stage
COPY --from=builder /app/target/release/ai-runtime /usr/local/bin/ai-runtime

# Set ownership and permissions
RUN chown ai-runtime:ai-runtime /usr/local/bin/ai-runtime && \
    chmod +x /usr/local/bin/ai-runtime

USER ai-runtime

EXPOSE 8084

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8084/health || exit 1

CMD ["ai-runtime"]

