FROM nvidia/cuda:12.2.0-cudnn8-runtime-ubuntu22.04

ENV DEBIAN_FRONTEND=noninteractive
ENV PYTHONUNBUFFERED=1

RUN apt-get update && apt-get install -y \
    python3.10 \
    python3-pip \
    git \
    wget \
    && rm -rf /var/lib/apt/lists/*

RUN pip3 install --upgrade pip

# Install JAX with GPU support
RUN pip3 install \
    "jax[cuda12]"==0.4.20 \
    jaxlib==0.4.20 \
    flax==0.7.5 \
    optax==0.1.7 \
    numpy==1.24.3 \
    transformers==4.35.0 \
    requests==2.31.0

WORKDIR /workspace

COPY train.py /workspace/
COPY infer.py /workspace/

ENV MODEL_PATH=/model
ENV DATA_PATH=/data
ENV OUTPUT_PATH=/tmp/artha/output

CMD ["python3", "train.py"]

