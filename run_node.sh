#!/bin/bash
# ============================================================
# ArthaChain Community Node Runner
# Run your own ArthaChain node - No staking, no special hardware!
# ============================================================

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

print_banner() {
    echo -e "${BLUE}"
    echo "╔═══════════════════════════════════════════════════════════╗"
    echo "║            🚀 ArthaChain Community Node 🚀                ║"
    echo "║                                                           ║"
    echo "║   Run your own node - No staking required!                ║"
    echo "║   Standard ports: 8080 (API) | 8545 (EVM) | 8084 (P2P)   ║"
    echo "╚═══════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

detect_gpu() {
    if command -v nvidia-smi &> /dev/null; then
        echo -e "${GREEN}✅ GPU Detected! Your node can participate in AI compute tasks.${NC}"
        export ENABLE_GPU=true
    else
        echo -e "${YELLOW}ℹ️  No GPU detected. Running as standard node.${NC}"
        export ENABLE_GPU=false
    fi
}

check_ports() {
    echo -e "${BLUE}🔍 Checking port availability...${NC}"
    
    PORTS=(8080 8545 9944 8084 9184)
    PORTS_OK=true
    
    for port in "${PORTS[@]}"; do
        if lsof -Pi :$port -sTCP:LISTEN -t >/dev/null 2>&1; then
            echo -e "${RED}❌ Port $port is already in use${NC}"
            PORTS_OK=false
        fi
    done
    
    if [ "$PORTS_OK" = false ]; then
        echo ""
        echo -e "${RED}Please free up the ports above or stop conflicting services.${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}✅ All ports available${NC}"
}

start_node() {
    print_banner
    detect_gpu
    check_ports
    
    echo ""
    echo -e "${GREEN}🚀 Starting ArthaChain Node...${NC}"
    echo ""
    
    docker compose -f docker-compose.community.yml up -d
    
    echo ""
    echo -e "${GREEN}✅ Your ArthaChain node is running!${NC}"
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo -e "${BLUE}📡 Standard Endpoints (same for all nodes):${NC}"
    echo "   REST API:     http://localhost:8080"
    echo "   EVM RPC:      http://localhost:8545  (MetaMask compatible)"
    echo "   gRPC:         localhost:9944"
    echo "   P2P Network:  localhost:8084"
    echo "   Metrics:      http://localhost:9184/metrics"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo -e "${YELLOW}💡 Node Types (auto-selected based on your hardware):${NC}"
    echo "   • Standard Node  - Block validation"
    echo "   • GPU Node       - AI compute tasks (if GPU detected)"
    echo "   • Mining Node    - Block creation"
    echo "   • Validator Node - Block finality"
    echo "   • Shard Node     - Cross-shard processing (auto-assigned)"
    echo ""
    echo -e "${GREEN}Run './run_node.sh logs' to see node activity${NC}"
}

stop_node() {
    print_banner
    echo -e "${YELLOW}🛑 Stopping node...${NC}"
    docker compose -f docker-compose.community.yml down
    echo -e "${GREEN}✅ Node stopped${NC}"
}

status_node() {
    print_banner
    
    echo -e "${BLUE}📊 Node Status:${NC}"
    docker compose -f docker-compose.community.yml ps
    
    echo ""
    if curl -s -f http://localhost:8080/health > /dev/null 2>&1; then
        echo -e "${GREEN}✅ Node is healthy and running${NC}"
        
        # Try to get node info
        echo ""
        echo -e "${BLUE}📈 Node Info:${NC}"
        curl -s http://localhost:8080/api/v1/node/info 2>/dev/null | jq . 2>/dev/null || echo "   (API response pending...)"
    else
        echo -e "${RED}❌ Node is not responding${NC}"
    fi
}

logs_node() {
    docker compose -f docker-compose.community.yml logs -f
}

help() {
    print_banner
    echo "Usage: ./run_node.sh [command]"
    echo ""
    echo "Commands:"
    echo "  start    Start your ArthaChain node"
    echo "  stop     Stop the node"
    echo "  restart  Restart the node"
    echo "  status   Check node health"
    echo "  logs     View node logs"
    echo "  clean    Stop and remove all data"
    echo "  help     Show this help"
    echo ""
    echo -e "${BLUE}Standard Ports (same for everyone):${NC}"
    echo "  8080  - REST API"
    echo "  8545  - EVM JSON-RPC (MetaMask)"
    echo "  9944  - gRPC"
    echo "  8084  - P2P Network"
    echo "  9184  - Prometheus Metrics"
}

case "${1:-help}" in
    start)
        start_node
        ;;
    stop)
        stop_node
        ;;
    restart)
        stop_node
        sleep 2
        start_node
        ;;
    status)
        status_node
        ;;
    logs)
        logs_node
        ;;
    clean)
        print_banner
        echo -e "${RED}🗑️  Removing node and all data...${NC}"
        docker compose -f docker-compose.community.yml down -v
        echo -e "${GREEN}✅ Cleanup complete${NC}"
        ;;
    help|*)
        help
        ;;
esac
