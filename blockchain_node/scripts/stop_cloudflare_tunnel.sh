#!/bin/bash

# ArthaChain Cloudflare Tunnel Stopper
# Stops the Cloudflare tunnel for ArthaChain services

echo "🛑 Stopping ArthaChain Cloudflare Tunnel"

# Check if PID file exists
if [ ! -f /tmp/cloudflared.pid ]; then
    echo "⚠️  Cloudflare tunnel PID file not found"
    # Try to find running tunnel processes
    TUNNEL_PIDS=$(pgrep -f "cloudflared.*tunnel.*run" 2>/dev/null)
    if [ -z "$TUNNEL_PIDS" ]; then
        echo "✅ No Cloudflare tunnel processes found"
        exit 0
    else
        echo "🔍 Found running tunnel processes: $TUNNEL_PIDS"
        for PID in $TUNNEL_PIDS; do
            echo "🛑 Killing process $PID"
            kill $PID
        done
        # Wait a moment for processes to terminate
        sleep 2
        # Force kill any remaining processes
        for PID in $TUNNEL_PIDS; do
            if kill -0 $PID 2>/dev/null; then
                echo "💥 Force killing process $PID"
                kill -9 $PID
            fi
        done
        echo "✅ Cloudflare tunnel stopped"
        exit 0
    fi
fi

# Read PID from file
TUNNEL_PID=$(cat /tmp/cloudflared.pid)

# Check if process is still running
if ! kill -0 $TUNNEL_PID 2>/dev/null; then
    echo "⚠️  Cloudflare tunnel process (PID: $TUNNEL_PID) is not running"
    rm -f /tmp/cloudflared.pid
    exit 0
fi

# Gracefully stop the tunnel
echo "🛑 Stopping Cloudflare tunnel (PID: $TUNNEL_PID)"
kill $TUNNEL_PID

# Wait for process to terminate
echo "⏳ Waiting for tunnel to stop..."
sleep 5

# Check if process is still running
if kill -0 $TUNNEL_PID 2>/dev/null; then
    echo "💥 Force killing Cloudflare tunnel (PID: $TUNNEL_PID)"
    kill -9 $TUNNEL_PID
fi

# Clean up PID file
rm -f /tmp/cloudflared.pid

echo "✅ Cloudflare tunnel stopped successfully"