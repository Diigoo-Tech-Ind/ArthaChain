# AI Proofs Service
FROM rust:1.82-slim as builder

WORKDIR /app

# Copy workspace root files
COPY Cargo.toml Cargo.lock ./

# Copy service-specific Cargo.toml
COPY services/ai-proofs/Cargo.toml ./services/ai-proofs/

# Copy source code
COPY services/ai-proofs ./services/ai-proofs

# Build dependencies first (caching layer)
RUN cargo build --release --bin ai-proofs || true

# Build the application
RUN cargo build --release --bin ai-proofs

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
RUN useradd -r -s /bin/false ai-proofs

# Copy binary from builder stage
COPY --from=builder /app/target/release/ai-proofs /usr/local/bin/ai-proofs

# Set ownership and permissions
RUN chown ai-proofs:ai-proofs /usr/local/bin/ai-proofs && \
    chmod +x /usr/local/bin/ai-proofs

USER ai-proofs

EXPOSE 8085

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8085/health || exit 1

CMD ["ai-proofs"]

