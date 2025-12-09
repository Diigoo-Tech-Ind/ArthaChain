#!/bin/bash
# ArthaChain SDK Publishing Script
# Publishes arthajs to npm and arthapy to PyPI

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo -e "${BLUE}"
echo "╔═══════════════════════════════════════════════════════════╗"
echo "║          ArthaChain SDK Publisher                         ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo -e "${NC}"

publish_npm() {
    echo -e "${BLUE}📦 Publishing arthajs to npm...${NC}"
    cd "$SCRIPT_DIR/arthajs"
    
    # Build
    echo "  Building TypeScript..."
    npm run build
    
    # Publish
    echo "  Publishing to npm..."
    npm publish --access public
    
    echo -e "${GREEN}✅ arthajs published to npm!${NC}"
    echo "   Install: npm install @arthachain/sdk"
}

publish_pypi() {
    echo -e "${BLUE}📦 Publishing arthapy to PyPI...${NC}"
    cd "$SCRIPT_DIR/arthapy"
    
    # Clean previous builds
    rm -rf dist/ build/ *.egg-info/
    
    # Build
    echo "  Building Python package..."
    python -m build
    
    # Upload to PyPI
    echo "  Uploading to PyPI..."
    python -m twine upload dist/*
    
    echo -e "${GREEN}✅ arthapy published to PyPI!${NC}"
    echo "   Install: pip install arthapy"
}

help() {
    echo "Usage: ./publish.sh [command]"
    echo ""
    echo "Commands:"
    echo "  npm       Publish arthajs to npm"
    echo "  pypi      Publish arthapy to PyPI"
    echo "  all       Publish both SDKs"
    echo "  help      Show this help"
    echo ""
    echo "Prerequisites:"
    echo "  npm:  npm login (logged into npm registry)"
    echo "  pypi: pip install build twine (and PyPI credentials configured)"
}

case "${1:-help}" in
    npm)
        publish_npm
        ;;
    pypi)
        publish_pypi
        ;;
    all)
        publish_npm
        echo ""
        publish_pypi
        ;;
    help|*)
        help
        ;;
esac
