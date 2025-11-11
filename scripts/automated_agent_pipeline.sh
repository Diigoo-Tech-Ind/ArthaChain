#!/bin/bash
# Automated Agent Pipeline - Complete end-to-end
set -e

echo "🤖 ArthaAIN Automated Agent Pipeline"
echo "===================================="

NODE_URL="${ARTHA_NODE:-http://localhost:8080}"
AGENT_SPEC="${1:-./agent_spec.json}"
GOAL="${2:-Automate the task}"
TOOLS="${3:-search,storage,tx}"

# Step 1: Upload agent spec
echo ""
echo "📦 Step 1: Uploading agent specification..."
if [ ! -f "$AGENT_SPEC" ]; then
    echo "   Creating default agent spec..."
    cat > "$AGENT_SPEC" <<EOF
{
  "framework": "langchain",
  "model": "gpt-4",
  "memory_policy": "episodic",
  "tools": ["search", "storage", "tx"],
  "max_iterations": 10
}
EOF
fi

AGENT_CID=$(arthai storage push "$AGENT_SPEC" --replicas 3 --months 6 | grep -oP 'artha://\S+' | head -1)
if [ -z "$AGENT_CID" ]; then
    echo "❌ Failed to upload agent spec"
    exit 1
fi
echo "   ✅ Agent spec uploaded: $AGENT_CID"

# Step 2: Submit agent job
echo ""
echo "🚀 Step 2: Submitting agent job..."
JOB_ID=$(arthai agent run \
    --aiid "$AGENT_CID" \
    --goal "$GOAL" \
    --tools "$TOOLS" \
    --memory episodic \
    --budget 100 2>&1 | grep -oP 'job-\S+' | head -1)

if [ -z "$JOB_ID" ]; then
    echo "❌ Failed to submit agent job"
    exit 1
fi

echo "   ✅ Agent job submitted: $JOB_ID"
echo ""
echo "⏳ Waiting for agent completion..."

# Poll for completion
TIMEOUT=1800
ELAPSED=0
while [ $ELAPSED -lt $TIMEOUT ]; do
    STATUS=$(curl -s "$NODE_URL/ai/job/$JOB_ID/status" | grep -oP '"status"\s*:\s*"\K[^"]+' | head -1)
    
    case "$STATUS" in
        "Completed")
            echo ""
            echo "✅ Agent completed!"
            break
            ;;
        "Failed"|"Cancelled")
            echo ""
            echo "❌ Agent failed: $STATUS"
            exit 1
            ;;
        *)
            sleep 10
            ELAPSED=$((ELAPSED + 10))
            printf "\r   Elapsed: ${ELAPSED}s"
            ;;
    esac
done

# Get results
echo ""
echo "📥 Step 3: Retrieving agent results..."
RESULT=$(curl -s "$NODE_URL/ai/job/$JOB_ID/status" | jq -r '.artifacts[]' 2>/dev/null || echo "")
if [ -n "$RESULT" ]; then
    echo "   ✅ Results available"
    echo "$RESULT"
fi

echo ""
echo "✅ Agent pipeline complete!"
echo "   Job: $JOB_ID"

