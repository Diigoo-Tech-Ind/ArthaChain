#!/bin/bash

# ArthaChain Node Status Checker
# Simple script for community members to check node status

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if node is running
check_node_status() {
    if systemctl is-active --quiet arthachain-node; then
        print_success "ArthaChain node is running"
        return 0
    else
        print_error "ArthaChain node is not running"
        return 1
    fi
}

# Function to check disk space
check_disk_space() {
    local usage=$(df /opt/arthachain | tail -1 | awk '{print $5}' | sed 's/%//')
    if [ "$usage" -gt 90 ]; then
        print_error "Disk usage is critical: ${usage}%"
    elif [ "$usage" -gt 80 ]; then
        print_warning "Disk usage is high: ${usage}%"
    else
        print_success "Disk usage is normal: ${usage}%"
    fi
}

# Function to check memory usage
check_memory_usage() {
    local usage=$(free | grep Mem | awk '{printf("%.0f", $3/$2 * 100.0)}')
    if [ "$usage" -gt 90 ]; then
        print_error "Memory usage is critical: ${usage}%"
    elif [ "$usage" -gt 80 ]; then
        print_warning "Memory usage is high: ${usage}%"
    else
        print_success "Memory usage is normal: ${usage}%"
    fi
}

# Function to check network connectivity
check_network() {
    # Check if required ports are listening
    local ports=(30303 8080 8545 9184 1900)
    for port in "${ports[@]}"; do
        if netstat -tuln | grep -q ":$port "; then
            print_success "Port $port is open"
        else
            print_warning "Port $port is not open"
        fi
    done
}

# Function to check recent logs
check_recent_logs() {
    print_status "Recent log entries:"
    journalctl -u arthachain-node -n 10 --no-pager
}

# Function to check blockchain height
check_blockchain_height() {
    if curl -s -f http://localhost:8080/health >/dev/null; then
        local height=$(curl -s http://localhost:8080/api/v1/blockchain/height | jq -r '.height' 2>/dev/null)
        if [ -n "$height" ] && [ "$height" != "null" ]; then
            print_success "Blockchain height: $height"
        else
            print_warning "Unable to retrieve blockchain height"
        fi
    else
        print_warning "Node API is not responding"
    fi
}

# Function to check peer connections
check_peers() {
    if curl -s -f http://localhost:8080/health >/dev/null; then
        local peers=$(curl -s http://localhost:8080/api/v1/network/peers | jq -r '.peers | length' 2>/dev/null)
        if [ -n "$peers" ] && [ "$peers" != "null" ]; then
            print_success "Connected peers: $peers"
        else
            print_warning "Unable to retrieve peer information"
        fi
    else
        print_warning "Node API is not responding"
    fi
}

# Main function
main() {
    echo "========================================="
    echo "    ArthaChain Node Status Checker"
    echo "========================================="
    echo ""
    
    # Check node status
    check_node_status
    echo ""
    
    # Check system resources
    print_status "System Resources:"
    check_disk_space
    check_memory_usage
    echo ""
    
    # Check network
    print_status "Network Status:"
    check_network
    echo ""
    
    # Check blockchain status
    print_status "Blockchain Status:"
    check_blockchain_height
    check_peers
    echo ""
    
    # Show recent logs
    check_recent_logs
    echo ""
    
    echo "========================================="
    echo "For detailed logs: journalctl -u arthachain-node -f"
    echo "To start/stop node: sudo systemctl {start|stop|restart} arthachain-node"
    echo "========================================="
}

# Run main function
main "$@"