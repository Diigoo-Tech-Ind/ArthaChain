# ArthaAIN v1 - PyTorch Runtime
# Supports PyTorch, HuggingFace Transformers, vLLM for training and inference

FROM nvidia/cuda:12.2.0-cudnn8-runtime-ubuntu22.04

# Install Python and system dependencies
RUN apt-get update && apt-get install -y \
    python3.10 \
    python3-pip \
    git \
    wget \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install PyTorch with CUDA support
RUN pip3 install --no-cache-dir \
    torch==2.1.0 \
    torchvision==0.16.0 \
    torchaudio==2.1.0 \
    --index-url https://download.pytorch.org/whl/cu121

# Install ML libraries
RUN pip3 install --no-cache-dir \
    transformers==4.35.0 \
    accelerate==0.24.0 \
    datasets==2.14.0 \
    tokenizers==0.15.0 \
    sentencepiece==0.1.99 \
    protobuf==3.20.3 \
    tensorboard==2.15.0 \
    wandb==0.16.0 \
    numpy==1.24.3 \
    scipy==1.11.3 \
    scikit-learn==1.3.2 \
    pandas==2.1.3

# Install vLLM for fast inference
RUN pip3 install --no-cache-dir vllm==0.2.6

# Install Artha SVDB client
RUN pip3 install --no-cache-dir requests blake3

# Create working directories
RUN mkdir -p /model /data /checkpoints /app

# Copy training/inference scripts
COPY train.py /app/train.py
COPY infer.py /app/infer.py
COPY checkpoint_saver.py /app/checkpoint_saver.py

# Set environment variables
ENV PYTHONUNBUFFERED=1
ENV CUDA_VISIBLE_DEVICES=0
ENV TRANSFORMERS_CACHE=/tmp/transformers
ENV HF_HOME=/tmp/huggingface

# Default command (can be overridden)
CMD ["python3", "/app/train.py"]

