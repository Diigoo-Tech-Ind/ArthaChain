FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive
ENV PYTHONUNBUFFERED=1

RUN apt-get update && apt-get install -y \
    python3.10 \
    python3-pip \
    git \
    && rm -rf /var/lib/apt/lists/*

RUN pip3 install --upgrade pip

# Quantum computing bridge (forwards to QPU providers)
RUN pip3 install \
    qiskit==0.44.1 \
    cirq==1.2.0 \
    pennylane==0.32.0 \
    numpy==1.24.3 \
    requests==2.31.0

WORKDIR /workspace

COPY train.py /workspace/
COPY infer.py /workspace/
COPY quantum_bridge.py /workspace/

ENV MODEL_PATH=/model
ENV DATA_PATH=/data
ENV OUTPUT_PATH=/tmp/artha/output
ENV QPU_PROVIDER_URL=${QPU_PROVIDER_URL:-""}

CMD ["python3", "train.py"]

