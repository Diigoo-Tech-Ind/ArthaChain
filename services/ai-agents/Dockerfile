# AI Agents Service - Production Ready
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
COPY services/ai-agents/Cargo.toml ./services/ai-agents/

# Pre-build dependencies (caching optimization)
RUN cargo fetch

# Build dependencies only (caching layer)
RUN cargo build --release --target-dir /app/target || true

# Copy source code
COPY services/ai-agents ./services/ai-agents

# Build the application
RUN cargo build --release --bin ai-agents

# Strip binary to reduce size
RUN strip /app/target/release/ai-agents

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
RUN groupadd -r -g 1006 ai-agents && \
    useradd -r -u 1006 -g ai-agents -s /bin/false -d /nonexistent ai-agents

# Copy binary from builder stage
COPY --from=builder --chown=ai-agents:ai-agents /app/target/release/ai-agents /usr/local/bin/ai-agents

# Set executable permissions
RUN chmod +x /usr/local/bin/ai-agents

# Switch to non-root user
USER ai-agents

# Set working directory
WORKDIR /app

EXPOSE 8086

# Health check with proper intervals
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:8086/health || exit 1

# Use exec form for proper signal handling
CMD ["ai-agents"]
