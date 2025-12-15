#!/bin/bash
set -e

# =====================================================================
# ArthaChain Universal Node Entrypoint
# =====================================================================

# Default values if not set
export NODE_ROLE=${NODE_ROLE:-full}
export RUST_LOG=${RUST_LOG:-info}
export DATA_DIR=${DATA_DIR:-/data}

echo "🚀 Starting ArthaChain Node..."
echo "📍 Role: $NODE_ROLE"
echo "🔧 Data Dir: $DATA_DIR"

# Ensure data directory exists and is writable
mkdir -p "$DATA_DIR"

# Handle specific roles
case "$NODE_ROLE" in
    "auto")
        echo "🤖 Auto-Detecting Best Node Role..."
        
        # Check for GPU
        if command -v nvidia-smi &> /dev/null; then
            echo "   ✅ GPU Detected (NVIDIA)"
            echo "   👉 Selecting Role: AI Worker + Full Node"
            
            # Start AI Daemon in background
            echo "   🤖 Starting AI Job Daemon..."
            ai-jobd &
            AI_PID=$!
            
            # Start Full Node with Mining/Sharding
            echo "   🌟 Starting ArthaChain Smart Node (Mining + Sharding + Validation)..."
            exec arthachain_node \
                --validator \
                --enable-mining \
                --enable-sharding \
                --base-path "$DATA_DIR" \
                --chain testnet
        else
            echo "   ❌ No GPU Detected"
            echo "   👉 Selecting Role: Full Node (Mining + Validation)"
            
            echo "   🌟 Starting ArthaChain Smart Node..."
            exec arthachain_node \
                --validator \
                --enable-mining \
                --enable-sharding \
                --base-path "$DATA_DIR" \
                --chain testnet
        fi
        ;;

    "ai-worker"|"ai")
        echo "🤖 Starting AI Job Daemon..."
        exec ai-jobd
        ;;
        
    "api-gateway"|"gateway")
        echo "🌐 Starting API Gateway..."
        # If API_PORT is 3000, use that, otherwise default to 1930 or 8080?
        # api_server takes --port argument
        PORT=${API_PORT:-1930}
        NETWORK=${NETWORK:-mainnet}
        echo "   Port: $PORT"
        echo "   Network: $NETWORK"
        exec api_server --port "$PORT" --network "$NETWORK"
        ;;
    
    "validator")
        echo "🛡️ Starting Validator Node..."
        exec arthachain_node \
            --validator \
            --base-path "$DATA_DIR" \
            --chain testnet
        ;;
        
    "miner")
        echo "⛏️ Starting Miner Node..."
        # Mining is usually a flag on the full node
        export MINING_ENABLED=true
        exec arthachain_node \
            --validator \
            --base-path "$DATA_DIR" \
            --chain testnet
        ;;
        
    "rpc"|"api")
        echo "🔌 Starting RPC/API Node..."
        exec arthachain_node \
            --rpc-external \
            --ws-external \
            --rpc-cors all \
            --base-path "$DATA_DIR" \
            --chain testnet
        ;;
        
    "p2p-relay")
        echo "📡 Starting P2P Relay Node..."
        exec arthachain_node \
            --p2p-only \
            --base-path "$DATA_DIR" \
            --chain testnet
        ;;

    *)
        echo "🌟 Starting Standard Full Node..."
        # Pass through any command line arguments
        exec arthachain_node \
            --base-path "$DATA_DIR" \
            --chain testnet \
            "$@"
        ;;
esac
