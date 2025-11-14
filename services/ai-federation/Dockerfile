# AI Federation Service
FROM rust:1.82-slim as builder

WORKDIR /app

# Copy workspace root files
COPY Cargo.toml Cargo.lock ./

# Copy service-specific Cargo.toml
COPY services/ai-federation/Cargo.toml ./services/ai-federation/

# Copy source code
COPY services/ai-federation ./services/ai-federation

# Build dependencies first (caching layer)
RUN cargo build --release --bin ai-federation || true

# Build the application
RUN cargo build --release --bin ai-federation

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
RUN useradd -r -s /bin/false ai-federation

# Copy binary from builder stage
COPY --from=builder /app/target/release/ai-federation /usr/local/bin/ai-federation

# Set ownership and permissions
RUN chown ai-federation:ai-federation /usr/local/bin/ai-federation && \
    chmod +x /usr/local/bin/ai-federation

USER ai-federation

EXPOSE 8087

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8087/health || exit 1

CMD ["ai-federation"]

