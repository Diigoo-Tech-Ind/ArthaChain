#!/bin/bash
# ArthaChain Multi-Node Testnet Management Script
# Usage: ./run_testnet.sh [command]

set -e

COMPOSE_FILE="testnet-multi-node.yml"
PROJECT_NAME="arthachain-testnet"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_banner() {
    echo -e "${BLUE}"
    echo "╔═══════════════════════════════════════════════════════════╗"
    echo "║         ArthaChain Multi-Node Testnet Manager             ║"
    echo "╚═══════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

start() {
    print_banner
    echo -e "${GREEN}🚀 Starting 3-node testnet...${NC}"
    docker compose -f $COMPOSE_FILE -p $PROJECT_NAME up -d
    echo ""
    echo -e "${GREEN}✅ Testnet started!${NC}"
    echo ""
    echo "📊 Node Endpoints:"
    echo "   Validator 1: http://localhost:8080 (API) | http://localhost:8545 (EVM RPC)"
    echo "   Validator 2: http://localhost:8081 (API) | http://localhost:8546 (EVM RPC)"
    echo "   Validator 3: http://localhost:8082 (API) | http://localhost:8547 (EVM RPC)"
    echo ""
    echo "📈 Monitoring:"
    echo "   Prometheus:  http://localhost:9090"
    echo "   Grafana:     http://localhost:3000 (admin/arthachain)"
    echo ""
    echo "Run './run_testnet.sh logs' to see node logs"
}

stop() {
    print_banner
    echo -e "${YELLOW}🛑 Stopping testnet...${NC}"
    docker compose -f $COMPOSE_FILE -p $PROJECT_NAME down
    echo -e "${GREEN}✅ Testnet stopped${NC}"
}

restart() {
    stop
    start
}

logs() {
    docker compose -f $COMPOSE_FILE -p $PROJECT_NAME logs -f
}

logs_validator() {
    if [ -z "$1" ]; then
        echo "Usage: ./run_testnet.sh logs-validator [1|2|3]"
        exit 1
    fi
    docker compose -f $COMPOSE_FILE -p $PROJECT_NAME logs -f validator-$1
}

status() {
    print_banner
    echo -e "${BLUE}📊 Testnet Status:${NC}"
    docker compose -f $COMPOSE_FILE -p $PROJECT_NAME ps
    echo ""
    
    echo -e "${BLUE}🔗 Node Health Checks:${NC}"
    for i in 1 2 3; do
        port=$((8079 + i))
        if curl -s -f "http://localhost:$port/health" > /dev/null 2>&1; then
            echo -e "   Validator $i: ${GREEN}✅ Healthy${NC}"
        else
            echo -e "   Validator $i: ${RED}❌ Unhealthy or Not Running${NC}"
        fi
    done
}

clean() {
    print_banner
    echo -e "${RED}🗑️  Cleaning up testnet (removing volumes)...${NC}"
    docker compose -f $COMPOSE_FILE -p $PROJECT_NAME down -v
    echo -e "${GREEN}✅ Cleanup complete${NC}"
}

build() {
    print_banner
    echo -e "${BLUE}🔨 Building Docker images...${NC}"
    docker compose -f $COMPOSE_FILE -p $PROJECT_NAME build
    echo -e "${GREEN}✅ Build complete${NC}"
}

test_consensus() {
    print_banner
    echo -e "${BLUE}🧪 Testing Consensus...${NC}"
    
    # Check all nodes are healthy
    for i in 1 2 3; do
        port=$((8079 + i))
        if ! curl -s -f "http://localhost:$port/health" > /dev/null 2>&1; then
            echo -e "${RED}❌ Validator $i is not healthy. Cannot test consensus.${NC}"
            exit 1
        fi
    done
    
    echo -e "${GREEN}✅ All validators healthy${NC}"
    
    # Get block heights from all nodes
    echo ""
    echo "📊 Block Heights:"
    for i in 1 2 3; do
        port=$((8079 + i))
        height=$(curl -s "http://localhost:$port/api/v1/blocks/latest" 2>/dev/null | jq -r '.height' 2>/dev/null || echo "N/A")
        echo "   Validator $i: Block $height"
    done
    
    echo ""
    echo -e "${GREEN}✅ Consensus test complete${NC}"
}

help() {
    print_banner
    echo "Usage: ./run_testnet.sh [command]"
    echo ""
    echo "Commands:"
    echo "  start              Start the 3-node testnet"
    echo "  stop               Stop the testnet"
    echo "  restart            Restart the testnet"
    echo "  build              Build Docker images"
    echo "  logs               View all node logs"
    echo "  logs-validator N   View logs for validator N (1, 2, or 3)"
    echo "  status             Check testnet status"
    echo "  test-consensus     Test consensus across nodes"
    echo "  clean              Stop and remove all volumes"
    echo "  help               Show this help message"
}

# Main command handling
case "${1:-help}" in
    start)
        start
        ;;
    stop)
        stop
        ;;
    restart)
        restart
        ;;
    build)
        build
        ;;
    logs)
        logs
        ;;
    logs-validator)
        logs_validator $2
        ;;
    status)
        status
        ;;
    test-consensus)
        test_consensus
        ;;
    clean)
        clean
        ;;
    help|*)
        help
        ;;
esac
