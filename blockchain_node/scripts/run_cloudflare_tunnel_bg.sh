#!/bin/bash

# ArthaChain Cloudflare Tunnel Runner (Background)
# Starts the Cloudflare tunnel for ArthaChain services in the background

set -e

echo "🚀 Starting ArthaChain Cloudflare Tunnel in Background"

# Check if cloudflared is installed
if ! command -v cloudflared &> /dev/null; then
    echo "❌ cloudflared is not installed"
    echo "Please install cloudflared first:"
    echo "  macOS: brew install cloudflared"
    echo "  Linux: Visit https://developers.cloudflare.com/cloudflare-one/connections/connect-apps/install-and-setup/installation/"
    exit 1
fi

# Check if credentials file exists
CREDENTIALS_FILE="/Users/sainathtangallapalli/.cloudflared/cc3311b0-2a5c-4444-aacc-668576634499.json"
if [ ! -f "$CREDENTIALS_FILE" ]; then
    echo "❌ Cloudflare credentials file not found: $CREDENTIALS_FILE"
    echo "Please ensure you have the correct credentials file from Cloudflare Zero Trust dashboard"
    exit 1
fi

# Check if config file exists
CONFIG_FILE="/Users/sainathtangallapalli/blockchain/ArthaChain/blockchain_node/cloudflared/config.yml"
if [ ! -f "$CONFIG_FILE" ]; then
    echo "❌ Cloudflare tunnel config file not found: $CONFIG_FILE"
    exit 1
fi

# Verify the tunnel configuration
echo "🔍 Verifying tunnel configuration..."
if cloudflared tunnel --config "$CONFIG_FILE" validate; then
    echo "✅ Tunnel configuration is valid"
else
    echo "❌ Tunnel configuration is invalid"
    exit 1
fi

# Check if tunnel is already running
if pgrep -f "cloudflared.*tunnel.*run" > /dev/null; then
    echo "⚠️  Cloudflare tunnel is already running"
    echo "To stop it, run: pkill -f 'cloudflared.*tunnel.*run'"
    exit 1
fi

# Start the tunnel in background
echo "🚇 Starting Cloudflare tunnel in background..."
echo "   Config: $CONFIG_FILE"
echo "   Credentials: $CREDENTIALS_FILE"

# Run the tunnel in the background
nohup cloudflared tunnel --config "$CONFIG_FILE" run > /tmp/cloudflared.log 2>&1 &
TUNNEL_PID=$!

# Save PID to file
echo $TUNNEL_PID > /tmp/cloudflared.pid

echo "✅ Cloudflare tunnel started successfully with PID: $TUNNEL_PID"
echo "📝 Logs are being written to: /tmp/cloudflared.log"
echo "🛑 To stop the tunnel, run: ./scripts/stop_cloudflare_tunnel.sh"