# AI Job Daemon Service
FROM rust:1.82-slim as builder

WORKDIR /app

# Copy workspace root files
COPY Cargo.toml Cargo.lock ./

# Copy service-specific Cargo.toml
COPY services/ai-jobd/Cargo.toml ./services/ai-jobd/

# Copy source code
COPY services/ai-jobd ./services/ai-jobd

# Build dependencies first (caching layer)
RUN cargo build --release --bin ai-jobd || true

# Build the application
RUN cargo build --release --bin ai-jobd

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
RUN useradd -r -s /bin/false ai-jobd

# Copy binary from builder stage
COPY --from=builder /app/target/release/ai-jobd /usr/local/bin/ai-jobd

# Set ownership and permissions
RUN chown ai-jobd:ai-jobd /usr/local/bin/ai-jobd && \
    chmod +x /usr/local/bin/ai-jobd

USER ai-jobd

EXPOSE 8081

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8081/health || exit 1

CMD ["ai-jobd"]

