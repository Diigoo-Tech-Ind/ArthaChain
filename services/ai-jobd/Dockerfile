# AI Job Daemon Service - Production Ready
# Updated: 2025 - Latest stable versions with security best practices

# Build stage
FROM rust:1.83-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace root files
COPY Cargo.toml Cargo.lock ./

# Copy service-specific Cargo.toml
COPY services/ai-jobd/Cargo.toml ./services/ai-jobd/

# Pre-build dependencies (caching optimization)
RUN cargo fetch

# Build dependencies only (caching layer)
RUN cargo build --release --target-dir /app/target || true

# Copy source code
COPY services/ai-jobd ./services/ai-jobd

# Build the application
RUN cargo build --release --bin ai-jobd

# Strip binary to reduce size
RUN strip /app/target/release/ai-jobd

# Runtime stage
FROM debian:bookworm-slim as runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    libgcc-s1 \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* \
    && rm -rf /tmp/* \
    && rm -rf /var/tmp/*

# Create non-root user with specific UID/GID
RUN groupadd -r -g 1002 ai-jobd && \
    useradd -r -u 1002 -g ai-jobd -s /bin/false -d /nonexistent ai-jobd

# Copy binary from builder stage
COPY --from=builder --chown=ai-jobd:ai-jobd /app/target/release/ai-jobd /usr/local/bin/ai-jobd

# Set executable permissions
RUN chmod +x /usr/local/bin/ai-jobd

# Switch to non-root user
USER ai-jobd

# Set working directory
WORKDIR /app

EXPOSE 8081

# Health check with proper intervals
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:8081/health || exit 1

# Use exec form for proper signal handling
CMD ["ai-jobd"]

