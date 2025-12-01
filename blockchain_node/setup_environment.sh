#!/bin/bash
# ArthaChain Environment Setup Script
# This script configures the perfect environment for blockchain node operation

set -e

echo "🚀 ArthaChain Environment Setup"
echo "================================"

# 1. Configure Cargo to use local directory (not external drive)
echo "📦 Configuring Cargo environment..."
export CARGO_HOME="$HOME/.cargo"
export PATH="$HOME/.cargo/bin:$PATH"

# Create cargo directory if it doesn't exist
mkdir -p "$CARGO_HOME"

# 2. Configure Rust logging
echo "📝 Setting up logging..."
export RUST_LOG=info
export RUST_BACKTRACE=1

# 3. Configure node networking
echo "🌐 Configuring network settings..."
export NODE_ADDRESS="0.0.0.0:8080"
export P2P_LISTEN_ADDR="0.0.0.0:30303"
export RPC_LISTEN_ADDR="0.0.0.0:8545"
export WEBSOCKET_LISTEN_ADDR="0.0.0.0:8546"
export GRPC_ADDR="0.0.0.0:9944"
export PROMETHEUS_ENDPOINT="0.0.0.0:9090"

# 4. Configure blockchain parameters
echo "⛓️  Setting blockchain parameters..."
export ARTHA_CHAIN_ID=1
export ARTHA_NETWORK="mainnet"
export ARTHA_DATA_DIR="./data"
export ARTHA_GENESIS_PATH="./genesis.json"

# 5. AI Services Configuration (Optional - set if using AI features)
echo "🤖 AI Services configuration (optional)..."
# Uncomment and set these if you have AI services
# export ARTHA_JOBD_URL="http://localhost:8001"
# export ARTHA_RUNTIME_URL="http://localhost:8002"
# export ARTHA_FEDERATION_URL="http://localhost:8003"
# export ARTHA_EVOLUTION_URL="http://localhost:8004"
# export ARTHA_AGENTS_URL="http://localhost:8005"

# 6. Storage Configuration for SVDB
echo "💾 Configuring storage..."
export ARTHA_STORAGE_PATH="./data/storage"
export ARTHA_DB_PATH="./data/db"
export ARTHA_SVDB_URL="http://localhost:3000"

# 7. Security Configuration
echo "🔒 Configuring security..."
export ARTHA_ENABLE_QUANTUM_CRYPTO=true
export ARTHA_ENABLE_DDOS_PROTECTION=true

# 8. Performance Configuration
echo "⚡ Configuring performance..."
export ARTHA_MAX_CONNECTIONS=1000
export ARTHA_WORKER_THREADS=16
export ARTHA_TARGET_TPS=100000

# 9. Validator Configuration (set if running as validator)
echo "🛡️  Validator configuration..."
# Uncomment if running as validator
# export ARTHA_ROLE_VALIDATOR=true
# export ARTHA_VALIDATOR_KEY="your_validator_private_key_here"

# 10. Storage Provider Configuration (set if providing storage)
echo "📦 Storage provider configuration..."
# Uncomment if running as storage provider
# export ARTHA_ROLE_SP=true
# export ARTHA_PROVIDER="0xYourProviderAddress"
# export ARTHA_PRIVATE_KEY="your_private_key_for_proofs"

# 11. Create necessary directories
echo "📁 Creating data directories..."
mkdir -p "$ARTHA_DATA_DIR"
mkdir -p "$ARTHA_STORAGE_PATH"
mkdir -p "$ARTHA_DB_PATH"
mkdir -p "./logs"

# 12. Verify environment
echo ""
echo "✅ Environment Setup Complete!"
echo "================================"
echo "Cargo Home: $CARGO_HOME"
echo "Data Directory: $ARTHA_DATA_DIR"
echo "Network: $ARTHA_NETWORK"
echo "API Gateway: http://$NODE_ADDRESS"
echo "RPC Service: http://$RPC_LISTEN_ADDR"
echo "WebSocket RPC: ws://$WEBSOCKET_LISTEN_ADDR"
echo "P2P Network: tcp://$P2P_LISTEN_ADDR"
echo "Prometheus: http://$PROMETHEUS_ENDPOINT"
echo "gRPC: http://$GRPC_ADDR"
echo ""
echo "Next steps:"
echo "1. Source this file: source ./setup_environment.sh"
echo "2. Build the project: cargo build --release --all-targets"
echo "3. Run the node: ./target/release/arthachain-node"
echo ""
