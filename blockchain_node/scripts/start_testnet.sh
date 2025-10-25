#!/bin/bash

# ArthaChain Testnet Startup Script
# Starts multiple nodes for local testing

set -e

echo "🚀 Starting ArthaChain Testnet"

# Create data directories for each node
mkdir -p data/node1 data/node2 data/node3

# Function to start a node
start_node() {
    local node_id=$1
    local port_offset=$2
    local api_port=$((8080 + port_offset))
    local p2p_port=$((30303 + port_offset))
    local data_dir="data/node$node_id"
    
    echo "Starting Node $node_id on API port $api_port, P2P port $p2p_port"
    
    # Create node config
    cat > "config/node$node_id.yaml" << EOF
# Node $node_id Configuration
data_dir: "$data_dir"
network:
  p2p_port: $p2p_port
  max_peers: 50
  bootstrap_nodes:
    - "/ip4/127.0.0.1/tcp/30303"
    - "/ip4/127.0.0.1/tcp/30304"
    - "/ip4/127.0.0.1/tcp/30305"
  network_name: "arthachain-testnet"
  connection_timeout: 30
  discovery_enabled: true
  version: "1.0.0"
consensus:
  block_time: 15
  max_block_size: 5242880
  consensus_type: "svbft"
  difficulty_adjustment_period: 2016
  reputation_decay_rate: 0.05
storage:
  db_type: "rocksdb"
  max_open_files: 512
  db_path: "$data_dir/db"
  svdb_url: "http://localhost:3000"
  size_threshold: 1048576
api:
  enabled: true
  port: $api_port
  host: "127.0.0.1"
  address: "127.0.0.1"
  cors_domains: ["*"]
  allow_origin: ["*"]
  max_request_body_size: 10485760
  max_connections: 100
  enable_websocket: false
  enable_graphql: false
sharding:
  enabled: false
  shard_count: 1
  primary_shard: 0
  shard_id: 0
  cross_shard_timeout: 30
  assignment_strategy: "static"
  cross_shard_strategy: "atomic"
node_id: "node$node_id"
log_level: "info"
enable_metrics: true
metrics_addr: "127.0.0.1:910$port_offset"
enable_ai: false
ai_model_dir: "./models"
is_genesis: $([ $node_id -eq 1 ] && echo "true" || echo "false")
enable_api: true
genesis_path: "./genesis.json"
EOF

    # Start the node in background
    RUST_LOG=info ./target/release/arthachain_node --config "config/node$node_id.yaml" &
    echo $! > "node$node_id.pid"
    
    # Give node time to start
    sleep 2
}

# Build the project if not already built
if [ ! -f "./target/release/arthachain_node" ]; then
    echo "Building ArthaChain node..."
    cargo build --release
fi

# Start 3 nodes
start_node 1 0
start_node 2 1
start_node 3 2

echo ""
echo "✅ Testnet started with 3 nodes:"
echo "   Node 1: API=http://localhost:8080, P2P=/ip4/127.0.0.1/tcp/30303"
echo "   Node 2: API=http://localhost:8081, P2P=/ip4/127.0.0.1/tcp/30304"
echo "   Node 3: API=http://localhost:8082, P2P=/ip4/127.0.0.1/tcp/30305"
echo ""
echo "🔧 To stop the testnet, run: ./scripts/stop_testnet.sh"
echo "📊 To check node status, run: curl http://localhost:8080/health"