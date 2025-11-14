# Policy Gate Service - Production Ready
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
COPY services/policy-gate/Cargo.toml ./services/policy-gate/

# Pre-build dependencies (caching optimization)
RUN cargo fetch

# Build dependencies only (caching layer)
RUN cargo build --release --target-dir /app/target || true

# Copy source code
COPY services/policy-gate ./services/policy-gate

# Build the application
RUN cargo build --release --bin policy-gate

# Strip binary to reduce size
RUN strip /app/target/release/policy-gate

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
RUN groupadd -r -g 1005 policy-gate && \
    useradd -r -u 1005 -g policy-gate -s /bin/false -d /nonexistent policy-gate

# Copy binary from builder stage
COPY --from=builder --chown=policy-gate:policy-gate /app/target/release/policy-gate /usr/local/bin/policy-gate

# Set executable permissions
RUN chmod +x /usr/local/bin/policy-gate

# Switch to non-root user
USER policy-gate

# Set working directory
WORKDIR /app

EXPOSE 8082

# Health check with proper intervals
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:8082/health || exit 1

# Use exec form for proper signal handling
CMD ["policy-gate"]
