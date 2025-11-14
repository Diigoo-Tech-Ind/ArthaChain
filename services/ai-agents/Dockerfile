# AI Agents Service
FROM rust:1.82-slim as builder

WORKDIR /app

# Copy workspace root files
COPY Cargo.toml Cargo.lock ./

# Copy service-specific Cargo.toml
COPY services/ai-agents/Cargo.toml ./services/ai-agents/

# Copy source code
COPY services/ai-agents ./services/ai-agents

# Build dependencies first (caching layer)
RUN cargo build --release --bin ai-agents || true

# Build the application
RUN cargo build --release --bin ai-agents

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
RUN useradd -r -s /bin/false ai-agents

# Copy binary from builder stage
COPY --from=builder /app/target/release/ai-agents /usr/local/bin/ai-agents

# Set ownership and permissions
RUN chown ai-agents:ai-agents /usr/local/bin/ai-agents && \
    chmod +x /usr/local/bin/ai-agents

USER ai-agents

EXPOSE 8086

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8086/health || exit 1

CMD ["ai-agents"]

