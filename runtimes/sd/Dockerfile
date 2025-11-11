FROM nvidia/cuda:12.2.0-cudnn8-runtime-ubuntu22.04

ENV DEBIAN_FRONTEND=noninteractive
ENV PYTHONUNBUFFERED=1

RUN apt-get update && apt-get install -y \
    python3.10 \
    python3-pip \
    git \
    && rm -rf /var/lib/apt/lists/*

RUN pip3 install --upgrade pip

# Stable Diffusion stack
RUN pip3 install \
    torch==2.1.0 \
    torchvision==0.16.0 \
    diffusers==0.21.4 \
    transformers==4.35.0 \
    accelerate==0.24.1 \
    xformers==0.0.22 \
    safetensors==0.3.3 \
    requests==2.31.0 \
    Pillow==10.0.1

WORKDIR /workspace

COPY train.py /workspace/
COPY infer.py /workspace/

ENV MODEL_PATH=/model
ENV DATA_PATH=/data
ENV OUTPUT_PATH=/tmp/artha/output

CMD ["python3", "train.py"]

