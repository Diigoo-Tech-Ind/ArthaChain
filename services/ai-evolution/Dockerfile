# AI Evolution Service
FROM rust:1.82-slim as builder

WORKDIR /app

# Copy workspace root files
COPY Cargo.toml Cargo.lock ./

# Copy service-specific Cargo.toml
COPY services/ai-evolution/Cargo.toml ./services/ai-evolution/

# Copy source code
COPY services/ai-evolution ./services/ai-evolution

# Build dependencies first (caching layer)
RUN cargo build --release --bin ai-evolution || true

# Build the application
RUN cargo build --release --bin ai-evolution

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
RUN useradd -r -s /bin/false ai-evolution

# Copy binary from builder stage
COPY --from=builder /app/target/release/ai-evolution /usr/local/bin/ai-evolution

# Set ownership and permissions
RUN chown ai-evolution:ai-evolution /usr/local/bin/ai-evolution && \
    chmod +x /usr/local/bin/ai-evolution

USER ai-evolution

EXPOSE 8088

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8088/health || exit 1

CMD ["ai-evolution"]

