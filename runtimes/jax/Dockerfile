# ArthaAIN v1 - JAX Runtime - Production Ready
# Supports JAX, Flax, Optax for ML training and inference
# Updated: 2025 - Latest stable versions

FROM nvidia/cuda:12.6.0-cudnn9-runtime-ubuntu22.04

ENV DEBIAN_FRONTEND=noninteractive
ENV PYTHONUNBUFFERED=1
ENV PYTHONDONTWRITEBYTECODE=1
ENV PIP_NO_CACHE_DIR=1
ENV PIP_DISABLE_PIP_VERSION_CHECK=1

# Install Python and system dependencies (minimal)
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3.12 \
    python3-pip \
    python3.12-venv \
    git \
    wget \
    ca-certificates \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* \
    && rm -rf /tmp/* \
    && rm -rf /var/tmp/*

# Upgrade pip and install build tools
RUN python3.12 -m pip install --no-cache-dir --upgrade pip setuptools wheel

# Install JAX with CUDA 12.6 support (latest stable)
RUN pip3 install --no-cache-dir \
    "jax[cuda12_local]"==0.4.28 \
    jaxlib==0.4.28 \
    flax==0.8.3 \
    optax==0.2.2 \
    numpy==2.2.1 \
    transformers==4.46.0 \
    requests==2.32.3

# Create non-root user
RUN groupadd -r -g 1010 mluser && \
    useradd -r -u 1010 -g mluser -s /bin/bash -d /home/mluser mluser

# Create working directories with proper permissions
RUN mkdir -p /workspace /model /data /tmp/artha/output && \
    chown -R mluser:mluser /workspace /model /data /tmp/artha

# Copy training/inference scripts
COPY --chown=mluser:mluser train.py /workspace/train.py
COPY --chown=mluser:mluser infer.py /workspace/infer.py

# Set environment variables
ENV MODEL_PATH=/model
ENV DATA_PATH=/data
ENV OUTPUT_PATH=/tmp/artha/output

# Switch to non-root user
USER mluser

# Set working directory
WORKDIR /workspace

# Health check
HEALTHCHECK --interval=60s --timeout=30s --start-period=120s --retries=3 \
    CMD python3 -c "import jax; print('OK')" || exit 1

# Use exec form for proper signal handling
CMD ["python3", "train.py"]

