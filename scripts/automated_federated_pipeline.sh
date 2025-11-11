#!/bin/bash
# Automated Federated Learning Pipeline
set -e

echo "🔗 ArthaAIN Federated Learning Pipeline"
echo "========================================"

NODE_URL="${ARTHA_NODE:-http://localhost:8080}"
MODEL_ID="${1:-model-default}"
DATASETS="${2:-dataset-1,dataset-2}"
ROUNDS="${3:-10}"
DP_ENABLED="${4:-true}"
BUDGET="${5:-500}"

echo ""
echo "🚀 Starting federated learning..."
echo "   Model:   $MODEL_ID"
echo "   Datasets: $DATASETS"
echo "   Rounds:  $ROUNDS"
echo "   DP:      $DP_ENABLED"

# Submit federated job
FED_ID=$(arthai fed start \
    --model "$MODEL_ID" \
    --datasets "$DATASETS" \
    --rounds "$ROUNDS" \
    --dp "$DP_ENABLED" \
    --budget "$BUDGET" 2>&1 | grep -oP 'fed-\S+' | head -1)

if [ -z "$FED_ID" ]; then
    echo "❌ Failed to start federated learning"
    exit 1
fi

echo "   ✅ Federated learning started: $FED_ID"
echo ""
echo "⏳ Monitoring progress..."

# Monitor status
TIMEOUT=7200
ELAPSED=0
while [ $ELAPSED -lt $TIMEOUT ]; do
    STATUS=$(curl -s "$NODE_URL/ai/federated/$FED_ID/status" | grep -oP '"status"\s*:\s*"\K[^"]+' | head -1)
    CURRENT_ROUND=$(curl -s "$NODE_URL/ai/federated/$FED_ID/status" | grep -oP '"currentRound"\s*:\s*\K\d+' | head -1 || echo "0")
    
    case "$STATUS" in
        "Completed")
            echo ""
            echo "✅ Federated learning completed!"
            echo "   Final round: $CURRENT_ROUND"
            break
            ;;
        "Failed")
            echo ""
            echo "❌ Federated learning failed"
            exit 1
            ;;
        *)
            printf "\r   Round: %d/%s | Elapsed: %ds" "$CURRENT_ROUND" "$ROUNDS" "$ELAPSED"
            sleep 30
            ELAPSED=$((ELAPSED + 30))
            ;;
    esac
done

# Get aggregated model
echo ""
echo "📥 Retrieving aggregated model..."
AGGREGATED_CID=$(curl -s "$NODE_URL/ai/federated/$FED_ID/status" | grep -oP '"aggregatedModelCid"\s*:\s*"\K[^"]+' | head -1)
if [ -n "$AGGREGATED_CID" ]; then
    echo "   ✅ Aggregated model: $AGGREGATED_CID"
fi

echo ""
echo "✅ Federated learning pipeline complete!"
echo "   Fed ID: $FED_ID"

