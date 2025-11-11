FROM nvidia/cuda:12.2.0-cudnn8-runtime-ubuntu22.04

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
RUN pip3 install \
    torch==2.1.0 \
    torchvision==0.16.0 \
    opencv-python==4.8.1.78 \
    ultralytics==8.0.196 \
    transformers==4.35.0 \
    timm==0.9.7 \
    detectron2 \
    requests==2.31.0

WORKDIR /workspace

COPY train.py /workspace/
COPY infer.py /workspace/

ENV MODEL_PATH=/model
ENV DATA_PATH=/data
ENV OUTPUT_PATH=/tmp/artha/output

CMD ["python3", "train.py"]

