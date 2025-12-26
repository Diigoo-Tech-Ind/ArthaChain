FROM tensorflow/tensorflow:2.15.0-gpu

ENV DEBIAN_FRONTEND=noninteractive
ENV PYTHONUNBUFFERED=1

RUN apt-get update && apt-get install -y \
    git \
    wget \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN pip install --upgrade pip

# Install additional libraries
RUN pip install \
    keras==2.15.0 \
    tensorflow-datasets==4.9.0 \
    transformers==4.35.0 \
    datasets==2.14.0 \
    numpy==1.24.3 \
    requests==2.31.0 \
    Pillow==10.0.1 \
    blake3

WORKDIR /workspace

COPY train.py /workspace/
COPY infer.py /workspace/
COPY checkpoint_saver.py /workspace/

ENV MODEL_PATH=/model
ENV DATA_PATH=/data
ENV CHECKPOINT_PATH=/checkpoints
ENV OUTPUT_PATH=/tmp/artha/output

CMD ["python3", "train.py"]

