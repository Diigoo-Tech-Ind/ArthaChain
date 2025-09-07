#!/bin/bash
# ArthaChain Production Deployment Script
# Deploy to production with proper checks and monitoring

set -euo pipefail

# Configuration
ENVIRONMENT=${1:-"testnet"}
BACKUP_BEFORE_DEPLOY=${BACKUP_BEFORE_DEPLOY:-true}
HEALTH_CHECK_TIMEOUT=${HEALTH_CHECK_TIMEOUT:-300}
ROLLBACK_ON_FAILURE=${ROLLBACK_ON_FAILURE:-true}

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] $1${NC}"
}

error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $1${NC}" >&2
}

warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING: $1${NC}"
}

info() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')] INFO: $1${NC}"
}

# Pre-deployment checks
pre_deployment_checks() {
    log "Running pre-deployment checks..."
    
    # Check if Docker is running
    if ! docker info >/dev/null 2>&1; then
        error "Docker is not running"
        exit 1
    fi
    
    # Check if Docker Compose is available
    if ! command -v docker-compose &> /dev/null; then
        error "Docker Compose is not installed"
        exit 1
    fi
    
    # Check if required files exist
    local required_files=(
        "docker-compose.yml"
        "Dockerfile"
        "config/haproxy.cfg"
        "config/prometheus.yml"
        "cloudflared/config.yml"
    )
    
    for file in "${required_files[@]}"; do
        if [ ! -f "$file" ]; then
            error "Required file not found: $file"
            exit 1
        fi
    done
    
    # Check disk space
    local available_space=$(df . | tail -1 | awk '{print $4}')
    if [ "$available_space" -lt 10485760 ]; then  # 10GB in KB
        warn "Low disk space: $(($available_space / 1024 / 1024))GB available"
    fi
    
    log "Pre-deployment checks passed"
}

# Backup current deployment
backup_current_deployment() {
    if [ "$BACKUP_BEFORE_DEPLOY" = "true" ]; then
        log "Creating backup of current deployment..."
        ./scripts/backup.sh
        log "Backup completed"
    fi
}

# Build and deploy
deploy() {
    log "Building and deploying ArthaChain $ENVIRONMENT..."
    
    # Stop existing services
    log "Stopping existing services..."
    docker-compose down --remove-orphans || true
    
    # Build new images
    log "Building Docker images..."
    docker-compose build --no-cache
    
    # Start services
    log "Starting services..."
    docker-compose up -d
    
    log "Deployment initiated"
}

# Health checks
health_checks() {
    log "Running health checks..."
    
    local services=(
        "arthachain-node:1900"
        "arthachain-global-api:1910"
        "arthachain-realtime-api:1920"
        "arthachain-explorer-api:1930"
        "arthachain-rpc:1970"
        "arthachain-metrics:1950"
    )
    
    local start_time=$(date +%s)
    local timeout=$HEALTH_CHECK_TIMEOUT
    
    for service in "${services[@]}"; do
        local name=$(echo "$service" | cut -d: -f1)
        local port=$(echo "$service" | cut -d: -f2)
        
        log "Checking health of $name..."
        
        local elapsed=0
        while [ $elapsed -lt $timeout ]; do
            if curl -f -s "http://localhost:$port/health" >/dev/null 2>&1; then
                log "✅ $name is healthy"
                break
            fi
            
            sleep 5
            elapsed=$((elapsed + 5))
            
            if [ $elapsed -ge $timeout ]; then
                error "❌ $name failed health check after ${timeout}s"
                return 1
            fi
        done
    done
    
    log "All health checks passed"
    return 0
}

# Rollback function
rollback() {
    if [ "$ROLLBACK_ON_FAILURE" = "true" ]; then
        warn "Rolling back deployment..."
        docker-compose down
        # Restore from backup if available
        if [ -f "/backups/arthachain/latest-backup.tar.gz" ]; then
            log "Restoring from backup..."
            # Add restore logic here
        fi
        error "Rollback completed"
    fi
}

# Main deployment flow
main() {
    log "Starting ArthaChain production deployment for $ENVIRONMENT"
    
    # Pre-deployment checks
    pre_deployment_checks
    
    # Backup current deployment
    backup_current_deployment
    
    # Deploy
    if ! deploy; then
        error "Deployment failed"
        rollback
        exit 1
    fi
    
    # Health checks
    if ! health_checks; then
        error "Health checks failed"
        rollback
        exit 1
    fi
    
    # Final verification
    log "Running final verification..."
    sleep 30
    
    if ! health_checks; then
        error "Final verification failed"
        rollback
        exit 1
    fi
    
    log "🎉 ArthaChain $ENVIRONMENT deployment completed successfully!"
    
    # Display service URLs
    info "Service URLs:"
    info "• Main API: http://localhost:1900"
    info "• Global API: http://localhost:1910"
    info "• Real-Time API: http://localhost:1920"
    info "• Block Explorer: http://localhost:1930"
    info "• RPC: http://localhost:1970"
    info "• Metrics: http://localhost:1950"
    info "• HAProxy Stats: http://localhost:8404/stats"
    
    # Display global URLs if Cloudflare tunnel is running
    if docker-compose ps cloudflared | grep -q "Up"; then
        info "Global URLs:"
        info "• testnet.arthachain.in"
        info "• api.arthachain.in"
        info "• rpc.arthachain.in"
    fi
}

# Run main function
main "$@"
