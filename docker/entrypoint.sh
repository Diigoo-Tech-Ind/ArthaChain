#!/bin/bash
set -e

# =====================================================================
# ArthaChain Universal Node Entrypoint
# =====================================================================

# Default values if not set
export NODE_ROLE=${NODE_ROLE:-full}
export RUST_LOG=${RUST_LOG:-info}
# Note: DATA_DIR is not currently configurable via CLI in the binary, 
# it presumably uses internal defaults or config file
export API_PORT=${API_PORT:-1900}
export P2P_PORT=${P2P_PORT:-8084}
export METRICS_PORT=${METRICS_PORT:-9184}

echo "🚀 Starting ArthaChain Node..."
echo "📍 Role: $NODE_ROLE"
echo "🔌 API Port: $API_PORT"
echo "🌐 P2P Port: $P2P_PORT"

# Ensure simple execution for now, ignoring complex role logic that requires unsupported flags
# We pass the standard ports and enable all features for the testnet node

case "$NODE_ROLE" in
    "ai-worker"|"ai")
        echo "🤖 Starting AI Job Daemon..."
        exec ai-jobd
        ;;
        
    "api-gateway"|"gateway")
        echo "🌐 Starting API Gateway..."
        exec api_server --port "${API_PORT}" --network "testnet"
        ;;

    *)
        echo "🌟 Starting ArthaChain Smart Node (Unified Binary)..."
        echo "   (Mining, Consensus, and API enabled by default)"
        
        # Launch with supported arguments only
        exec arthachain_node \
            --api-port "$API_PORT" \
            --p2p-port "$P2P_PORT" \
            --metrics-port "$METRICS_PORT" \
            --enable-faucet \
            --enable-testnet-features \
            "$@"
        ;;
esac
