#!/bin/bash
# ArthaChain Production Startup Script
# Optimized for arthavhain.in hosting

set -euo pipefail

# Configuration
export ARTHACHAIN_ENV="production"
export RUST_LOG="info"
export RUST_BACKTRACE="1"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging function
log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Check system requirements
check_requirements() {
    log "Checking system requirements..."
    
    # Check available memory (minimum 4GB)
    memory_kb=$(grep MemTotal /proc/meminfo | awk '{print $2}')
    memory_gb=$((memory_kb / 1024 / 1024))
    
    if [ $memory_gb -lt 4 ]; then
        error "Insufficient memory: ${memory_gb}GB (minimum 4GB required)"
        exit 1
    fi
    
    # Check available disk space (minimum 50GB)
    disk_gb=$(df -BG . | tail -1 | awk '{print $4}' | sed 's/G//')
    
    if [ $disk_gb -lt 50 ]; then
        warn "Low disk space: ${disk_gb}GB available (50GB recommended)"
    fi
    
    success "System requirements check passed"
}

# Create necessary directories
create_directories() {
    log "Creating necessary directories..."
    
    mkdir -p data/rocksdb
    mkdir -p data/memmap
    mkdir -p logs
    mkdir -p config
    
    success "Directories created"
}

# Check and install dependencies
check_dependencies() {
    log "Checking dependencies..."
    
    # Check if running in Docker
    if [ -f /.dockerenv ]; then
        log "Running in Docker container"
        return 0
    fi
    
    # Check Rust installation
    if ! command -v cargo &> /dev/null; then
        error "Rust/Cargo not found. Please install Rust first."
        exit 1
    fi
    
    success "Dependencies check passed"
}

# Build the project
build_project() {
    log "Building ArthaChain node..."
    
    if [ -f /.dockerenv ]; then
        log "Skipping build in Docker (using pre-built binary)"
        return 0
    fi
    
    # Build in release mode for production
    cargo build --release --bins
    
    if [ $? -eq 0 ]; then
        success "Build completed successfully"
    else
        error "Build failed"
        exit 1
    fi
}

# Setup configuration
setup_config() {
    log "Setting up configuration..."
    
    # Use production config if available
    if [ -f "docker/production.toml" ]; then
        cp docker/production.toml config/production.toml
        export ARTHACHAIN_CONFIG="./config/production.toml"
    else
        warn "Production config not found, using default configuration"
    fi
    
    success "Configuration setup completed"
}

# Setup monitoring
setup_monitoring() {
    log "Setting up monitoring..."
    
    # Create prometheus config if not exists
    if [ ! -f "docker/prometheus.yml" ]; then
        cat > docker/prometheus.yml << EOF
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'arthachain'
    static_configs:
      - targets: ['localhost:9092']
    scrape_interval: 5s
    metrics_path: /metrics
EOF
    fi
    
    success "Monitoring setup completed"
}

# Pre-flight checks
preflight_checks() {
    log "Running pre-flight checks..."
    
    # Check ports availability
    ports=(8080 8545 9944 30303)
    for port in "${ports[@]}"; do
        if netstat -tuln | grep -q ":$port "; then
            warn "Port $port is already in use"
        fi
    done
    
    # Check network connectivity
    if ! ping -c 1 8.8.8.8 &> /dev/null; then
        warn "No internet connectivity detected"
    fi
    
    success "Pre-flight checks completed"
}

# Start the node
start_node() {
    log "Starting ArthaChain node..."
    
    # Set up signal handlers for graceful shutdown
    trap 'log "Shutting down..."; kill $NODE_PID 2>/dev/null; exit 0' SIGTERM SIGINT
    
    # Start the API server
    if [ -f /.dockerenv ]; then
        # Running in Docker
        exec ./testnet_api_server
    else
        # Running on host
        ./target/release/testnet_api_server &
        NODE_PID=$!
        
        success "ArthaChain node started (PID: $NODE_PID)"
        log "API Server: http://localhost:8080"
        log "JSON-RPC: http://localhost:8545"
        log "WebSocket: ws://localhost:9944"
        log "Health Check: http://localhost:8080/health"
        
        # Wait for the node process
        wait $NODE_PID
    fi
}

# Health check
health_check() {
    log "Performing health check..."
    
    # Wait for service to start
    sleep 10
    
    # Check API health
    if curl -f http://localhost:8080/health &> /dev/null; then
        success "Health check passed - Node is running"
        return 0
    else
        error "Health check failed - Node may not be responding"
        return 1
    fi
}

# Main execution
main() {
    log "🚀 Starting ArthaChain Production Node"
    log "==============================================="
    
    check_requirements
    create_directories  
    check_dependencies
    build_project
    setup_config
    setup_monitoring
    preflight_checks
    
    log "All checks passed - Starting node..."
    start_node
}

# Run with error handling
if [ "${BASH_SOURCE[0]}" == "${0}" ]; then
    main "$@"
fi
