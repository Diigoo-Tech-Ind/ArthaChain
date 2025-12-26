# ArthaAIN v1 - PyTorch Runtime - Production Ready
# Supports PyTorch, HuggingFace Transformers, vLLM for training and inference
# Updated: 2025 - Latest stable versions

FROM nvidia/cuda:12.1.1-cudnn8-runtime-ubuntu22.04

# Install Python and system dependencies (minimal)
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3.10 \
    python3-pip \
    python3.10-venv \
    git \
    wget \
    curl \
    ca-certificates \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* \
    && rm -rf /tmp/* \
    && rm -rf /var/tmp/*

# Upgrade pip and install build tools
RUN python3.10 -m pip install --no-cache-dir --upgrade pip setuptools wheel

# Install PyTorch with CUDA 12.1 support (vLLM compatible)
RUN pip3 install --no-cache-dir \
    torch==2.4.0 \
    torchvision==0.19.0 \
    torchaudio==2.4.0 \
    --index-url https://download.pytorch.org/whl/cu121

# Install ML libraries (latest stable versions)
RUN pip3 install --no-cache-dir \
    transformers==4.46.0 \
    accelerate==1.1.1 \
    datasets==3.1.0 \
    tokenizers==0.20.2 \
    sentencepiece==0.2.0 \
    protobuf==4.24.4 \
    tensorboard==2.17.0 \
    wandb==0.18.5 \
    numpy==1.26.4 \
    scipy==1.14.1 \
    scikit-learn==1.5.2 \
    pandas==2.2.3

# Install vLLM (now compatible with torch 2.4.0)
RUN pip3 install --no-cache-dir vllm==0.6.3

# Install Artha SVDB client
RUN pip3 install --no-cache-dir requests==2.32.3 blake3==1.0.0

# Create non-root user
RUN groupadd -r -g 1000 mluser && \
    useradd -r -u 1000 -g mluser -s /bin/bash -d /home/mluser mluser

# Create working directories with proper permissions
RUN mkdir -p /model /data /checkpoints /app /tmp/transformers /tmp/huggingface && \
    chown -R mluser:mluser /model /data /checkpoints /app /tmp/transformers /tmp/huggingface

# Copy training/inference scripts
COPY --chown=mluser:mluser train.py /app/train.py
COPY --chown=mluser:mluser infer.py /app/infer.py
COPY --chown=mluser:mluser checkpoint_saver.py /app/checkpoint_saver.py

# Set environment variables
ENV PYTHONUNBUFFERED=1
ENV PYTHONDONTWRITEBYTECODE=1
ENV CUDA_VISIBLE_DEVICES=0
ENV TRANSFORMERS_CACHE=/tmp/transformers
ENV HF_HOME=/tmp/huggingface
ENV PIP_NO_CACHE_DIR=1
ENV PIP_DISABLE_PIP_VERSION_CHECK=1

# Switch to non-root user
USER mluser

# Set working directory
WORKDIR /app

# Health check
HEALTHCHECK --interval=60s --timeout=30s --start-period=120s --retries=3 \
    CMD python3 -c "import torch; print('OK')" || exit 1

# Default command (can be overridden)
CMD ["python3", "/app/train.py"]

