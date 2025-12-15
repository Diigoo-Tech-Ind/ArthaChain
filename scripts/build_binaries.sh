#!/bin/bash
# =============================================================================
# ArthaChain Binary Builder
# =============================================================================
# This script builds all binaries locally. Run this ONCE before Docker builds.
# After running this, Docker builds will be instant (<2 minutes).
#
# Usage: ./scripts/build_binaries.sh
# =============================================================================

set -e

echo "🔨 ArthaChain Binary Builder"
echo "============================"
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Must run from ArthaChain root directory${NC}"
    exit 1
fi

echo "📦 Building release binaries with production features..."
echo "   This may take 10-15 minutes on first run."
echo ""

# Build all workspace binaries
cargo build --release --workspace --features production

echo ""
echo "✅ Build complete!"
echo ""

# Check which binaries were built
echo "📋 Built binaries:"
BINARIES=("arthachain_node" "ai-jobd" "api_server" "grpc_server")
for bin in "${BINARIES[@]}"; do
    if [ -f "target/release/$bin" ]; then
        SIZE=$(du -h "target/release/$bin" | cut -f1)
        echo -e "   ${GREEN}✓${NC} $bin ($SIZE)"
    else
        echo -e "   ${YELLOW}⚠${NC} $bin (not found)"
    fi
done

echo ""
echo "🐳 You can now build Docker images instantly:"
echo "   ./scripts/test_dockers.sh"
echo ""
