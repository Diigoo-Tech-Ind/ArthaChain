FROM nvidia/cuda:12.1.1-cudnn8-runtime-ubuntu22.04

ENV DEBIAN_FRONTEND=noninteractive
ENV PYTHONUNBUFFERED=1

RUN apt-get update && apt-get install -y \
    python3.10 \
    python3-pip \
    libopencv-dev \
    python3-opencv \
    git \
    && rm -rf /var/lib/apt/lists/*

RUN pip3 install --upgrade pip

# Computer Vision stack
# Computer Vision stack - Torch first
RUN pip3 install \
    torch==2.1.0 \
    torchvision==0.16.0 \
    requests==2.31.0 \
    "numpy<2.0.0" \
    blake3

# Detectron2 (requires torch)
RUN python3 -m pip install 'git+https://github.com/facebookresearch/detectron2.git' --no-build-isolation

# Other libs
RUN pip3 install \
    opencv-python==4.8.1.78 \
    ultralytics==8.0.196 \
    transformers==4.35.0 \
    timm==0.9.7 \
    "numpy<2.0.0"

WORKDIR /workspace

COPY train.py /workspace/
COPY infer.py /workspace/

ENV MODEL_PATH=/model
ENV DATA_PATH=/data
ENV OUTPUT_PATH=/tmp/artha/output

CMD ["python3", "train.py"]

