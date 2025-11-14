# Policy Gate Service
FROM rust:1.82-slim as builder

WORKDIR /app

# Copy workspace root files
COPY Cargo.toml Cargo.lock ./

# Copy service-specific Cargo.toml
COPY services/policy-gate/Cargo.toml ./services/policy-gate/

# Copy source code
COPY services/policy-gate ./services/policy-gate

# Build dependencies first (caching layer)
RUN cargo build --release --bin policy-gate || true

# Build the application
RUN cargo build --release --bin policy-gate

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
RUN useradd -r -s /bin/false policy-gate

# Copy binary from builder stage
COPY --from=builder /app/target/release/policy-gate /usr/local/bin/policy-gate

# Set ownership and permissions
RUN chown policy-gate:policy-gate /usr/local/bin/policy-gate && \
    chmod +x /usr/local/bin/policy-gate

USER policy-gate

EXPOSE 8082

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8082/health || exit 1

CMD ["policy-gate"]

