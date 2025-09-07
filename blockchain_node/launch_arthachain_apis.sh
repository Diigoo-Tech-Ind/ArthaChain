#!/bin/bash

# ArthaChain API Launch Script
# Port Scheme: 1900, 1910, 1920, 1930, 1940, 1950, 1960, 1970, 1980, 1990

set -e

echo "🚀 ArthaChain API Services Launch"
echo "📊 ArthaChain Port Scheme: 19xx Series"
echo ""

# Create logs directory
mkdir -p logs

# Kill any existing processes
echo "🔄 Stopping existing services..."
pkill -f "arthachain" || true
pkill -f "cargo.*run" || true

# Wait for processes to stop
sleep 2

# Function to start service with health check
start_service() {
    local name=$1
    local command=$2
    local port=$3
    local health_endpoint=$4
    
    echo "🚀 Starting $name on port $port..."
    
    # Check if port is available
    if lsof -Pi :$port -sTCP:LISTEN -t >/dev/null 2>&1; then
        echo "❌ Port $port is already in use"
        return 1
    fi
    
    # Start service in background
    eval "$command" > "logs/${name}.log" 2>&1 &
    local pid=$!
    echo $pid > "logs/${name}.pid"
    
    # Wait for service to start
    echo "⏳ Waiting for $name to start..."
    for i in {1..30}; do
        if curl -s --connect-timeout 2 "http://localhost:$port$health_endpoint" >/dev/null 2>&1; then
            echo "✅ $name is running on port $port"
            return 0
        fi
        sleep 1
    done
    
    echo "❌ $name failed to start on port $port"
    return 1
}

# Start ArthaChain Services

# 1. Main Blockchain Node (Port 1900) - Real API Server
echo "📦 Starting ArthaChain Node (Port 1900)..."
export CARGO_HOME=/tmp/cargo
export PORT=1900
cargo run --bin simple_api_server > logs/blockchain_node.log 2>&1 &
echo $! > logs/blockchain_node.pid

# 2. Global API Service (Port 1910) - Real API Server
echo "🌐 Starting Global API Service (Port 1910)..."
export PORT=1910
cargo run --bin simple_api_server > logs/global_api.log 2>&1 &
echo $! > logs/global_api.pid

# 3. Real-Time API Server (Port 1920) - Real API Server
echo "⚡ Starting Real-Time API Server (Port 1920)..."
export PORT=1920
cargo run --bin simple_api_server > logs/realtime_api.log 2>&1 &
echo $! > logs/realtime_api.pid

# 4. Block Explorer API (Port 1930) - Real API Server
echo "🔍 Starting Block Explorer API (Port 1930)..."
export PORT=1930
cargo run --bin simple_api_server > logs/explorer_api.log 2>&1 &
echo $! > logs/explorer_api.pid

# 5. Documentation Server (Port 1940)
echo "📚 Starting Documentation Server (Port 1940)..."
python3 -m http.server 1940 --directory docs > logs/docs.log 2>&1 &
echo $! > logs/docs.pid

# 6. Metrics Server (Port 1950)
echo "📊 Starting Metrics Server (Port 1950)..."
python3 -m http.server 1950 --directory . > logs/metrics.log 2>&1 &
echo $! > logs/metrics.pid

# 7. Faucet Service (Port 1960)
echo "🚰 Starting Faucet Service (Port 1960)..."
python3 -m http.server 1960 --directory . > logs/faucet.log 2>&1 &
echo $! > logs/faucet.pid

# 8. RPC Service (Port 1970)
echo "🔗 Starting RPC Service (Port 1970)..."
export PORT=1970
cargo run --bin rpc_server > logs/rpc.log 2>&1 &
echo $! > logs/rpc.pid

# 9. Wallet API Service (Port 1980)
echo "👛 Starting Wallet API Service (Port 1980)..."
export PORT=1980
cargo run --bin wallet_api_server > logs/wallet_api.log 2>&1 &
echo $! > logs/wallet_api.pid

# Wait for all services to start
echo ""
echo "⏳ Waiting for all services to start..."
sleep 20

# Health check all services
echo ""
echo "🏥 Health Check Results:"
echo "========================"

services=(
    "Blockchain Node:1900:/health"
    "Global API:1910:/health"
    "Real-Time API:1920:/health"
    "Block Explorer:1930:/health"
    "Documentation:1940:/"
    "Metrics:1950:/"
    "Faucet:1960:/"
    "RPC:1970:/health"
    "Wallet API:1980:/health"
)

for service in "${services[@]}"; do
    IFS=':' read -r name port endpoint <<< "$service"
    
    if curl -s --connect-timeout 5 "http://localhost:$port$endpoint" >/dev/null 2>&1; then
        echo "✅ $name (Port $port) - HEALTHY"
    else
        echo "❌ $name (Port $port) - UNHEALTHY"
    fi
done

echo ""
echo "🎯 ArthaChain API Services Status:"
echo "=================================="
echo "• Blockchain Node:    http://localhost:1900"
echo "• Global API:         http://localhost:1910"
echo "• Real-Time API:      http://localhost:1920"
echo "• Block Explorer:     http://localhost:1930"
echo "• Documentation:      http://localhost:1940"
echo "• Metrics:            http://localhost:1950"
echo "• Faucet:             http://localhost:1960"
echo "• RPC:                http://localhost:1970"
echo "• Wallet API:         http://localhost:1980"
echo ""
echo "🌍 Global Access (via Cloudflare Tunnel):"
echo "• testnet.arthachain.in:1900"
echo "• api.arthachain.in:1910"
echo "• ws.arthachain.in:1920"
echo "• explorer.arthachain.in:1930"
echo "• docs.arthachain.in:1940"
echo "• metrics.arthachain.in:1950"
echo "• faucet.arthachain.in:1960"
echo "• rpc.arthachain.in:1970"
echo "• wallet.arthachain.in:1980"
echo ""
echo "📋 Process IDs saved in logs/*.pid"
echo "📝 Logs available in logs/*.log"
echo ""
echo "🚀 ArthaChain APIs launched successfully!"
