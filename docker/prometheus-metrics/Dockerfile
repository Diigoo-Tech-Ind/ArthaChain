# ArthaChain Prometheus Metrics - Production Ready
# Role: Metrics collection, aggregation, and alerting
# Updated: 2025

FROM prom/prometheus:v2.48.0

# Labels
LABEL maintainer="ArthaChain Team <team@arthachain.io>"
LABEL description="Prometheus metrics server for ArthaChain blockchain"

# Copy Prometheus configuration
COPY prometheus.yml /etc/prometheus/prometheus.yml

# Copy alert rules if they exist
# COPY alerts.yml /etc/prometheus/alerts.yml

# Expose port
EXPOSE 9090

# Data volume
VOLUME ["/prometheus"]

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:9090/-/healthy || exit 1

# Entry point
ENTRYPOINT ["/bin/prometheus"]
CMD ["--config.file=/etc/prometheus/prometheus.yml", \
    "--storage.tsdb.path=/prometheus", \
    "--storage.tsdb.retention.time=30d", \
    "--web.console.libraries=/usr/share/prometheus/console_libraries", \
    "--web.console.templates=/usr/share/prometheus/consoles", \
    "--web.enable-lifecycle"]
