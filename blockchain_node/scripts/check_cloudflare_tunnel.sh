#!/bin/bash

# ArthaChain Cloudflare Tunnel Status Checker
# Checks the status of the Cloudflare tunnel for ArthaChain services

echo "🔍 Checking ArthaChain Cloudflare Tunnel Status"

# Check if tunnel is running by looking for the process
if pgrep -f "cloudflared.*tunnel.*run.*arthachain-testnet" > /dev/null; then
    echo "✅ Cloudflare tunnel is running"
    
    # Get tunnel info
    echo ""
    echo "📊 Tunnel Information:"
    cloudflared tunnel info arthachain-testnet
    
    # Check if services are accessible
    echo ""
    echo "🌐 Checking service accessibility:"
    
    # List of services to check
    SERVICES=(
        "testnet.arthachain.in:80"
        "api.arthachain.in:80"
        "ws.arthachain.in:80"
        "explorer.arthachain.in:80"
    )
    
    for service in "${SERVICES[@]}"; do
        if nc -z -w5 $(echo $service | cut -d: -f1) $(echo $service | cut -d: -f2) 2>/dev/null; then
            echo "   ✅ $service is accessible"
        else
            echo "   ⚠️  $service is not accessible (may be because local services aren't running)"
        fi
    done
    
else
    echo "❌ Cloudflare tunnel is not running"
    echo "To start the tunnel, run: ./scripts/run_cloudflare_tunnel_bg.sh"
fi