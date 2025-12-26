#!/bin/bash
echo "Verifying AI Runtimes..."

check_image() {
    IMAGE=$1
    CMD=$2
    echo -n "Testing $IMAGE... "
    if sudo docker run --rm --gpus all $IMAGE python3 -c "$CMD" > /dev/null 2>&1; then
        echo "✅ OK"
    else
        echo "❌ FAILED"
        # Try capturing output for debugging
        sudo docker run --rm --gpus all $IMAGE python3 -c "$CMD" || true
    fi
}

# Core
check_image "arthachain/runtime-torch:latest" "import torch; assert torch.cuda.is_available()"
check_image "arthachain/runtime-tf:latest" "import tensorflow as tf; assert len(tf.config.list_physical_devices('GPU')) > 0"
check_image "arthachain/runtime-jax:latest" "import jax; assert jax.devices()[0].platform == 'gpu'"

# Specialized
check_image "arthachain/runtime-sd:latest" "import torch; assert torch.cuda.is_available()"
check_image "arthachain/runtime-quantum:latest" "import qiskit"
check_image "arthachain/runtime-agent:latest" "import langchain"
check_image "arthachain/runtime-rllib:latest" "import ray"
check_image "arthachain/runtime-cv:latest" "import cv2, torch; assert torch.cuda.is_available()"
check_image "arthachain/runtime-audio:latest" "import whisper, torch; assert torch.cuda.is_available()"
check_image "arthachain/runtime-evo:latest" "import neat"
