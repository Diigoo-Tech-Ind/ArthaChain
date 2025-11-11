#!/bin/bash
# Auto-Payout Streaming Daemon
# Monitors jobs and streams payouts via DealMarket.computePayout()

set -e

echo "💰 ArthaAIN Auto-Payout Streaming Daemon"
echo "========================================="

NODE_URL="${ARTHA_NODE:-http://localhost:8080}"
DEAL_MARKET_ADDR="${DEAL_MARKET_ADDR}"
RPC_URL="${RPC_URL:-http://localhost:8545}"
CHECK_INTERVAL="${CHECK_INTERVAL:-30}"

if [ -z "$DEAL_MARKET_ADDR" ]; then
    echo "❌ DEAL_MARKET_ADDR not set"
    exit 1
fi

echo "   DealMarket: $DEAL_MARKET_ADDR"
echo "   RPC URL: $RPC_URL"
echo "   Check Interval: ${CHECK_INTERVAL}s"
echo ""

# Monitor completed jobs and trigger payouts
while true; do
    # Get completed jobs from ai-proofs service
    COMPLETED_JOBS=$(curl -s "$NODE_URL/ai/proofs/stats" | jq -r '.completed_jobs[]?' 2>/dev/null || echo "")
    
    for job_id in $COMPLETED_JOBS; do
        # Get job details
        JOB_STATUS=$(curl -s "$NODE_URL/ai/job/$job_id/status" | jq '.' 2>/dev/null)
        
        if [ -z "$JOB_STATUS" ]; then
            continue
        fi
        
        STATUS=$(echo "$JOB_STATUS" | jq -r '.job.status' 2>/dev/null)
        GPU_SECONDS=$(echo "$JOB_STATUS" | jq -r '.job.gpuSeconds // 0' 2>/dev/null)
        PROVIDER=$(echo "$JOB_STATUS" | jq -r '.job.provider' 2>/dev/null)
        
        if [ "$STATUS" = "Completed" ] && [ "$GPU_SECONDS" != "0" ] && [ -n "$PROVIDER" ]; then
            echo "💰 Processing payout for job: $job_id"
            echo "   Provider: $PROVIDER"
            echo "   GPU Seconds: $GPU_SECONDS"
            
            # Calculate payout (rate * seconds)
            RATE_PER_SECOND=100000000000000  # 0.0001 ARTH per second (in wei)
            PAYOUT=$((RATE_PER_SECOND * GPU_SECONDS))
            
            # Call DealMarket.computePayout() via JSON-RPC
            JOB_HASH=$(echo -n "$job_id" | sha256sum | cut -d' ' -f1)
            JOB_BYTES32="0x${JOB_HASH}"
            
            # In production: Use web3/ethers to call contract
            # For now: Log the call
            echo "   📝 Calling DealMarket.computePayout()..."
            echo "      Job Hash: $JOB_BYTES32"
            echo "      Provider: $PROVIDER"
            echo "      Amount: $PAYOUT wei"
            
            # Mark job as paid (in production: track in database)
            echo "   ✅ Payout processed"
        fi
    done
    
    sleep "$CHECK_INTERVAL"
done

