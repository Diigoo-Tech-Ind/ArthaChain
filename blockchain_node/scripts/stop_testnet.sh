#!/bin/bash

# ArthaChain Testnet Stop Script
# Stops all running testnet nodes

echo "🛑 Stopping ArthaChain Testnet"

# Function to stop a node
stop_node() {
    local node_id=$1
    local pid_file="node$node_id.pid"
    
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if ps -p $pid > /dev/null; then
            echo "Stopping Node $node_id (PID: $pid)"
            kill $pid
            # Wait for process to terminate
            sleep 2
            # Force kill if still running
            if ps -p $pid > /dev/null; then
                echo "Force killing Node $node_id"
                kill -9 $pid
            fi
        else
            echo "Node $node_id (PID: $pid) is not running"
        fi
        rm -f "$pid_file"
    else
        echo "No PID file found for Node $node_id"
    fi
}

# Stop all nodes
stop_node 1
stop_node 2
stop_node 3

# Clean up
rm -f node*.pid

echo "✅ Testnet stopped"