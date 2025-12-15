#!/bin/bash
# =============================================================================
# ArthaChain Docker Verification Suite
# =============================================================================
# FAST MODE: Uses pre-built binaries. All builds complete in <5 minutes total.
# =============================================================================

set -e

echo "🐳 ArthaChain Docker Verification Suite"
echo "========================================"
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Check if binaries exist
check_binaries() {
    echo "🔍 Checking for pre-built binaries..."
    
    MISSING=0
    for bin in arthachain_node ai-jobd api_server grpc_server; do
        if [ ! -f "target/release/$bin" ]; then
            echo -e "   ${RED}✗${NC} $bin not found"
            MISSING=1
        else
            echo -e "   ${GREEN}✓${NC} $bin"
        fi
    done
    
    if [ $MISSING -eq 1 ]; then
        echo ""
        echo -e "${YELLOW}⚠️  Some binaries are missing!${NC}"
        echo "   Run: ./scripts/build_binaries.sh"
        echo ""
        echo "   Or build now? (y/n)"
        read -r response
        if [[ "$response" =~ ^[Yy]$ ]]; then
            echo "🔨 Building binaries..."
            cargo build --release --workspace --features production
        else
            exit 1
        fi
    fi
    echo ""
}

# Build a Docker image
build_image() {
    local name=$1
    local dockerfile=$2
    local tag="arthachain/$name:latest"
    
    echo "🏗️  Building $name..."
    START=$(date +%s)
    
    if docker build -t "$tag" -f "$dockerfile" . > /dev/null 2>&1; then
        END=$(date +%s)
        DURATION=$((END - START))
        echo -e "   ${GREEN}✓${NC} $name built in ${DURATION}s"
        return 0
    else
        echo -e "   ${RED}✗${NC} $name FAILED"
        return 1
    fi
}

# Main
cd "$(dirname "$0")/.."

check_binaries

echo "🚀 Building Docker Images..."
echo ""

TOTAL_START=$(date +%s)
FAILED=0

# Build all images
build_image "validator" "docker/validator/Dockerfile" || FAILED=1
build_image "rpc" "docker/rpc/Dockerfile" || FAILED=1
build_image "ai" "docker/ai/Dockerfile" || FAILED=1
build_image "p2p" "docker/p2p/Dockerfile" || FAILED=1
build_image "storage" "docker/storage/Dockerfile" || FAILED=1
build_image "sentry" "docker/sentry/Dockerfile" || FAILED=1
build_image "api-gateway" "docker/api-gateway/Dockerfile" || FAILED=1
build_image "grpc" "docker/grpc/Dockerfile" || FAILED=1
build_image "websocket" "docker/websocket/Dockerfile" || FAILED=1
build_image "metrics" "docker/metrics/Dockerfile" || FAILED=1

TOTAL_END=$(date +%s)
TOTAL_DURATION=$((TOTAL_END - TOTAL_START))

echo ""
echo "========================================"
if [ $FAILED -eq 0 ]; then
    echo -e "✅ ${GREEN}All images built successfully!${NC}"
else
    echo -e "❌ ${RED}Some images failed to build${NC}"
fi
echo "⏱️  Total time: ${TOTAL_DURATION} seconds"
echo ""

# List images
echo "📦 Built Images:"
docker images | grep arthachain | head -10

exit $FAILED
