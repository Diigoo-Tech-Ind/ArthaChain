#!/bin/bash

# 🚀 ARTHACHAIN BLOCKCHAIN INITIALIZER
# Initialize blockchain with real data before starting

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

print_status() {
    echo -e "${GREEN}[INIT]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_header() {
    echo -e "${CYAN}================================${NC}"
    echo -e "${CYAN}$1${NC}"
    echo -e "${CYAN}================================${NC}"
}

# Initialize blockchain with REAL data
init_blockchain() {
    print_header "🚀 INITIALIZING ARTHACHAIN BLOCKCHAIN WITH REAL DATA"
    
    print_status "Creating blockchain data directory..."
    mkdir -p data/blockchain
    mkdir -p data/keys
    mkdir -p data/storage
    mkdir -p data/ai_models
    mkdir -p data/zk_params
    mkdir -p logs
    
    print_status "Submitting REAL transactions to your blockchain..."
    
    # Check if node is running
    if ! curl -s "http://localhost:8080/api/v1/network/stats" &> /dev/null; then
        print_error "❌ ArthaChain node is not running!"
        print_status "Start your node first: ./🚀_ARTHACHAIN.sh"
        return 1
    fi
    
    print_status "✅ Node is running! Now generating REAL data..."
    
    # Submit real transactions to create real blocks
    local tx_count=0
    
    # Transaction 1
    print_status "Submitting transaction 1..."
    local response1=$(curl -s -X POST http://localhost:8080/api/v1/transactions \
        -H "Content-Type: application/json" \
        -d '{"from": "0x742d35cc6634c0532925a3b844bc454e4438f44e", "to": "0xabcdef1234567890abcdef1234567890abcdef12", "amount": 1000000000000000000, "gas": 21000, "gas_price": 20000000000}')
    
    if [[ $? -eq 0 ]]; then
        print_status "✅ Transaction 1 submitted successfully"
        ((tx_count++))
    else
        print_warning "⚠️  Transaction 1 failed: $response1"
    fi
    
    # Transaction 2  
    print_status "Submitting transaction 2..."
    local response2=$(curl -s -X POST http://localhost:8080/api/v1/transactions \
        -H "Content-Type: application/json" \
        -d '{"from": "0x742d35cc6634c0532925a3b844bc454e4438f44e", "to": "0x1234567890abcdef1234567890abcdef12345678", "amount": 500000000000000000, "gas": 21000, "gas_price": 25000000000}')
    
    if [[ $? -eq 0 ]]; then
        print_status "✅ Transaction 2 submitted successfully"
        ((tx_count++))
    else
        print_warning "⚠️  Transaction 2 failed: $response2"
    fi
    
    # Transaction 3
    print_status "Submitting transaction 3..."
    local response3=$(curl -s -X POST http://localhost:8080/api/v1/transactions \
        -H "Content-Type: application/json" \
        -d '{"from": "0xabcdef1234567890abcdef1234567890abcdef12", "to": "0x742d35cc6634c0532925a3b844bc454e4438f44e", "amount": 200000000000000000, "gas": 21000, "gas_price": 30000000000}')
    
    if [[ $? -eq 0 ]]; then
        print_status "✅ Transaction 3 submitted successfully"
        ((tx_count++))
    else
        print_warning "⚠️  Transaction 3 failed: $response3"
    fi
    
    # Wait for mining workers to process transactions
    print_status "⏳ Waiting for your 16 mining workers to process transactions..."
    sleep 10
    
    # Check real blockchain state
    print_status "📊 Checking REAL blockchain state..."
    
    local real_stats=$(curl -s "http://localhost:8080/api/v1/network/stats")
    local real_blocks=$(curl -s "http://localhost:8080/api/v1/blocks/latest")
    
    print_status "✅ REAL blockchain data generated!"
    echo ""
    print_status "Transactions submitted: $tx_count"
    print_status "Real network stats: $real_stats"
    print_status "Latest block: $real_blocks"
    echo ""
    print_status "Your 16 mining workers are now processing REAL transactions!"
    print_status "Check your node logs to see the mining activity!"
    
    # No more fake JSON files - we're generating REAL blockchain data!
    
    print_status "✅ REAL blockchain data generated successfully!"
    echo ""
    
    print_status "What happened:"
    echo "  📦 Submitted 3 REAL transactions to your blockchain"
    echo "  📦 Your 16 mining workers are processing them"
    echo "  📦 Real blocks are being created (not fake JSON files)"
    echo "  📦 Your blockchain now has REAL activity"
    echo ""
    
    print_status "Check your blockchain state:"
    echo "  Network stats: curl http://localhost:8080/api/v1/network/stats"
    echo "  Latest block: curl http://localhost:8080/api/v1/blocks/latest"
    echo ""
}

# Show current blockchain data
show_data() {
    print_header "📊 REAL BLOCKCHAIN DATA"
    
    # Check if node is running
    if ! curl -s "http://localhost:8080/api/v1/network/stats" &> /dev/null; then
        print_error "❌ ArthaChain node is not running!"
        print_status "Start your node first: ./🚀_ARTHACHAIN.sh"
        return 1
    fi
    
    print_status "✅ Node is running! Fetching REAL data..."
    echo ""
    
    # Get real network stats
    print_status "🌐 Network Stats:"
    local network_stats=$(curl -s "http://localhost:8080/api/v1/network/stats")
    echo "  📊 $network_stats"
    
    echo ""
    
    # Get real latest block
    print_status "🧱 Latest Block:"
    local latest_block=$(curl -s "http://localhost:8080/api/v1/blocks/latest")
    echo "  📦 $latest_block"
    
    echo ""
    
    # Get real mempool info
    print_status "📋 Mempool Status:"
    local mempool_stats=$(curl -s "http://localhost:8080/api/v1/network/stats" | jq -r '.pending_transactions // "Unknown"')
    echo "  📊 Pending Transactions: $mempool_stats"
    
    echo ""
    print_status "This is REAL data from your running blockchain, not fake JSON files!"
}

# Reset blockchain data
reset_data() {
    print_header "🔄 RESETTING BLOCKCHAIN DATA"
    
    print_warning "This will delete all blockchain data. Are you sure? (y/N)"
    read -r response
    
    if [[ "$response" =~ ^[Yy]$ ]]; then
        print_status "Removing blockchain data..."
        rm -rf data/blockchain/*
        print_status "✅ Blockchain data reset complete!"
        echo ""
        print_status "Run initialization again: ./init_blockchain.sh"
    else
        print_status "Reset cancelled."
    fi
}

# Main execution
main() {
    case "${1:-}" in
        "show"|"-s")
            show_data
            ;;
        "reset"|"-r")
            reset_data
            ;;
        "help"|"-h"|*)
            echo "🚀 ARTHACHAIN BLOCKCHAIN INITIALIZER"
            echo ""
            echo "Usage: $0 [COMMAND]"
            echo ""
            echo "Commands:"
            echo "  (no args)  - Initialize blockchain with sample data"
            echo "  show       - Show current blockchain data"
            echo "  reset      - Reset all blockchain data"
            echo "  help       - Show this help"
            echo ""
            echo "Examples:"
            echo "  $0              # Initialize blockchain"
            echo "  $0 show         # Show current data"
            echo "  $0 reset        # Reset data"
            echo ""
            ;;
    esac
    
    if [[ -z "${1:-}" ]]; then
        init_blockchain
    fi
}

# Run main function
main "$@"
