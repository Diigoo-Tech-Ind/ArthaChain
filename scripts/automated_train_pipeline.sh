#!/bin/bash
# Automated Training Pipeline - End-to-end integration
set -e

echo "🚀 ArthaAIN Automated Training Pipeline"
echo "========================================"

# Configuration
NODE_URL="${ARTHA_NODE:-http://localhost:8080}"
OUTPUT_DIR="${OUTPUT_DIR:-./output}"

# Step 1: Upload dataset
echo ""
echo "📦 Step 1: Uploading dataset to SVDB..."
DATASET_FILE="${1:-./data/dataset.tar.gz}"
if [ ! -f "$DATASET_FILE" ]; then
    echo "⚠️  Dataset file not found: $DATASET_FILE"
    echo "   Creating dummy dataset..."
    mkdir -p ./data
    tar czf "$DATASET_FILE" ./data 2>/dev/null || echo "{}" > "$DATASET_FILE"
fi

DATASET_CID=$(arthai storage push "$DATASET_FILE" --replicas 5 --months 12 | grep -oP 'artha://\S+' | head -1)
if [ -z "$DATASET_CID" ]; then
    echo "❌ Failed to upload dataset"
    exit 1
fi
echo "   ✅ Dataset uploaded: $DATASET_CID"

# Step 2: Register dataset
echo ""
echo "📝 Step 2: Registering dataset on-chain..."
DATASET_ID=$(arthai dataset register "$DATASET_CID" --tags training --license "$DATASET_CID" | grep -oP 'dataset-\S+' | head -1)
if [ -z "$DATASET_ID" ]; then
    echo "❌ Failed to register dataset"
    exit 1
fi
echo "   ✅ Dataset registered: $DATASET_ID"

# Step 3: Upload model
echo ""
echo "📦 Step 3: Uploading initial model to SVDB..."
MODEL_FILE="${2:-./models/initial_model.pt}"
if [ ! -f "$MODEL_FILE" ]; then
    echo "⚠️  Model file not found, creating placeholder..."
    mkdir -p ./models
    echo "placeholder" > "$MODEL_FILE"
fi

MODEL_CID=$(arthai storage push "$MODEL_FILE" --replicas 3 --months 6 | grep -oP 'artha://\S+' | head -1)
if [ -z "$MODEL_CID" ]; then
    echo "❌ Failed to upload model"
    exit 1
fi
echo "   ✅ Model uploaded: $MODEL_CID"

# Step 4: Register model
echo ""
echo "📝 Step 4: Registering model on-chain..."
MODEL_ID=$(arthai model register "$MODEL_CID" --arch llama --dataset "$DATASET_ID" --version v1 | grep -oP 'model-\S+' | head -1)
if [ -z "$MODEL_ID" ]; then
    echo "❌ Failed to register model"
    exit 1
fi
echo "   ✅ Model registered: $MODEL_ID"

# Step 5: Submit training job
echo ""
echo "🚀 Step 5: Submitting training job..."
JOB_OUTPUT="$OUTPUT_DIR/trained_model.pt"
JOB_ID=$(arthai train --model "$MODEL_ID" --data "$DATASET_ID" \
    --epochs 3 --batch 64 --lr 0.001 --budget 500 \
    --output "$JOB_OUTPUT" 2>&1 | grep -oP 'job-\S+' | head -1)

if [ -z "$JOB_ID" ]; then
    echo "❌ Failed to submit training job"
    exit 1
fi

echo "   ✅ Training job submitted: $JOB_ID"
echo ""
echo "⏳ Waiting for completion..."
echo "   (Monitor with: arthai job-status $JOB_ID)"

# Wait for completion
TIMEOUT=3600
ELAPSED=0
while [ $ELAPSED -lt $TIMEOUT ]; do
    STATUS=$(curl -s "$NODE_URL/ai/job/$JOB_ID/status" | grep -oP '"status"\s*:\s*"\K[^"]+' | head -1)
    
    case "$STATUS" in
        "Completed")
            echo ""
            echo "✅ Training completed!"
            break
            ;;
        "Failed"|"Cancelled")
            echo ""
            echo "❌ Training failed: $STATUS"
            exit 1
            ;;
        *)
            sleep 10
            ELAPSED=$((ELAPSED + 10))
            printf "\r   Elapsed: ${ELAPSED}s"
            ;;
    esac
done

if [ $ELAPSED -ge $TIMEOUT ]; then
    echo ""
    echo "⚠️  Training timed out after ${TIMEOUT}s"
    exit 1
fi

# Step 6: Get final model
echo ""
echo "📥 Step 6: Retrieving trained model..."
if [ -f "$JOB_OUTPUT" ]; then
    echo "   ✅ Model saved to: $JOB_OUTPUT"
else
    echo "   ⚠️  Model file not found"
fi

# Step 7: Deploy model (optional)
if [ "${DEPLOY_MODEL:-false}" = "true" ]; then
    echo ""
    echo "🚀 Step 7: Deploying model..."
    DEPLOY_RESULT=$(arthai deploy --model "$MODEL_ID" --endpoint /generate --replicas 1 --max-tokens 2048)
    echo "   ✅ $DEPLOY_RESULT"
fi

echo ""
echo "✅ Pipeline complete!"
echo "   Dataset: $DATASET_ID"
echo "   Model: $MODEL_ID"
echo "   Job: $JOB_ID"

