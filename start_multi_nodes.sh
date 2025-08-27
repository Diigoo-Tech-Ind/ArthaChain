#!/bin/bash

# ArthaChain Multi-Node Startup Script
# This script starts multiple nodes with different ports for testing sharding and parallel processing

echo "🚀 Starting ArthaChain Multi-Node Network..."
echo "=============================================="

# Kill any existing processes
echo "🧹 Cleaning up existing processes..."
pkill -f testnet_api_server || true
sleep 2

# Start Node 1 (Default ports)
echo ""
echo "🌐 Starting Node 1 (Default ports)..."
echo "   API Port: 8081"
echo "   P2P Port: 30303"
export CARGO_HOME=/tmp/cargo
export API_PORT=8081
export P2P_PORT=30303
cargo run --bin testnet_api_server > node1.log 2>&1 &
NODE1_PID=$!
echo "✅ Node 1 started with PID: $NODE1_PID"

# Wait for Node 1 to initialize
echo "⏳ Waiting for Node 1 to initialize..."
sleep 30

# Start Node 2 (Different ports)
echo ""
echo "🌐 Starting Node 2 (Different ports)..."
echo "   API Port: 8082"
echo "   P2P Port: 30304"
export API_PORT=8082
export P2P_PORT=30304
cargo run --bin testnet_api_server > node2.log 2>&1 &
NODE2_PID=$!
echo "✅ Node 2 started with PID: $NODE2_PID"

# Wait for Node 2 to initialize
echo "⏳ Waiting for Node 2 to initialize..."
sleep 30

# Start Node 3 (Different ports)
echo ""
echo "🌐 Starting Node 3 (Different ports)..."
echo "   API Port: 8083"
echo "   P2P Port: 30305"
export API_PORT=8083
export P2P_PORT=30305
cargo run --bin testnet_api_server > node3.log 2>&1 &
NODE3_PID=$!
echo "✅ Node 3 started with PID: $NODE3_PID"

# Wait for all nodes to initialize
echo "⏳ Waiting for all nodes to initialize..."
sleep 30

echo ""
echo "🎉 Multi-Node Network Started Successfully!"
echo "=============================================="
echo "Node 1: API http://localhost:8081, P2P 30303 (PID: $NODE1_PID)"
echo "Node 2: API http://localhost:8082, P2P 30304 (PID: $NODE2_PID)"
echo "Node 3: API http://localhost:8083, P2P 30305 (PID: $NODE3_PID)"
echo ""
echo "📊 Check node status:"
echo "   curl http://localhost:8081/health"
echo "   curl http://localhost:8082/health"
echo "   curl http://localhost:8083/health"
echo ""
echo "🔍 View logs:"
echo "   tail -f node1.log"
echo "   tail -f node2.log"
echo "   tail -f node3.log"
echo ""
echo "🛑 To stop all nodes: pkill -f testnet_api_server"
echo "=============================================="
