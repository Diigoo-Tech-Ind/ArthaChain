#!/bin/bash
# Automated Deployment Pipeline
set -e

echo "🚀 ArthaAIN Deployment Pipeline"
echo "================================"

NODE_URL="${ARTHA_NODE:-http://localhost:8080}"
MODEL_ID="${1:-model-default}"
ENDPOINT="${2:-/generate}"
REPLICAS="${3:-2}"
MAX_TOKENS="${4:-2048}"

echo ""
echo "📦 Step 1: Deploying model..."
echo "   Model:      $MODEL_ID"
echo "   Endpoint:   $ENDPOINT"
echo "   Replicas:   $REPLICAS"
echo "   Max Tokens: $MAX_TOKENS"

# Deploy model
DEPLOYMENT_ID=$(arthai deploy \
    --model "$MODEL_ID" \
    --endpoint "$ENDPOINT" \
    --replicas "$REPLICAS" \
    --max-tokens "$MAX_TOKENS" 2>&1 | grep -oP 'deploy-\S+' | head -1)

if [ -z "$DEPLOYMENT_ID" ]; then
    echo "❌ Failed to deploy model"
    exit 1
fi

echo "   ✅ Deployment started: $DEPLOYMENT_ID"
echo ""
echo "⏳ Waiting for deployment to be ready..."

# Wait for deployment
TIMEOUT=300
ELAPSED=0
while [ $ELAPSED -lt $TIMEOUT ]; do
    STATUS=$(curl -s "$NODE_URL/ai/deployment/$DEPLOYMENT_ID/status" | grep -oP '"status"\s*:\s*"\K[^"]+' | head -1)
    
    case "$STATUS" in
        "active"|"ready")
            echo ""
            echo "✅ Deployment ready!"
            ENDPOINT_URL=$(curl -s "$NODE_URL/ai/deployment/$DEPLOYMENT_ID/status" | grep -oP '"endpoint"\s*:\s*"\K[^"]+' | head -1)
            echo "   Endpoint: $ENDPOINT_URL"
            break
            ;;
        "failed")
            echo ""
            echo "❌ Deployment failed"
            exit 1
            ;;
        *)
            sleep 5
            ELAPSED=$((ELAPSED + 5))
            printf "\r   Elapsed: ${ELAPSED}s"
            ;;
    esac
done

# Test inference
echo ""
echo "🧪 Step 2: Testing inference endpoint..."
TEST_RESPONSE=$(curl -s -X POST "$ENDPOINT_URL" \
    -H "Content-Type: application/json" \
    -d '{"prompt": "Hello, ArthaChain!"}' || echo "")

if [ -n "$TEST_RESPONSE" ]; then
    echo "   ✅ Inference test successful"
else
    echo "   ⚠️  Inference test failed or endpoint not ready"
fi

echo ""
echo "✅ Deployment pipeline complete!"
echo "   Deployment: $DEPLOYMENT_ID"

