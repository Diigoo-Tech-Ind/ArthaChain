#!/bin/bash

# ArthaChain Cloudflare Tunnel Runner
# Starts the Cloudflare tunnel for ArthaChain services

set -e

echo "🚀 Starting ArthaChain Cloudflare Tunnel"

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

# Start the tunnel
echo "🚇 Starting Cloudflare tunnel..."
echo "   Config: $CONFIG_FILE"
echo "   Credentials: $CREDENTIALS_FILE"

# Run the tunnel in the foreground
cloudflared tunnel --config "$CONFIG_FILE" run

echo "✅ Cloudflare tunnel started successfully"