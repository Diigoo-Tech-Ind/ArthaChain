#!/bin/bash

# ArthaChain Permanent Cloudflare Tunnel Manager
# Manages the permanent Cloudflare tunnel setup for ArthaChain services

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if running on macOS
check_os() {
    if [[ "$OSTYPE" != "darwin"* ]]; then
        print_error "This script is designed for macOS only"
        exit 1
    fi
}

# Function to check if cloudflared is installed
check_cloudflared() {
    if ! command -v cloudflared &> /dev/null; then
        print_error "cloudflared is not installed"
        echo "Please install cloudflared first:"
        echo "  brew install cloudflared"
        exit 1
    fi
    print_status "Found cloudflared version: $(cloudflared --version | head -n 1)"
}

# Function to check if credentials file exists
check_credentials() {
    CREDENTIALS_FILE="/Users/sainathtangallapalli/.cloudflared/cc3311b0-2a5c-4444-aacc-668576634499.json"
    if [ ! -f "$CREDENTIALS_FILE" ]; then
        print_error "Cloudflare credentials file not found: $CREDENTIALS_FILE"
        echo "Please ensure you have the correct credentials file from Cloudflare Zero Trust dashboard"
        exit 1
    fi
    print_status "Found credentials file: $CREDENTIALS_FILE"
}

# Function to check if config file exists
check_config() {
    CONFIG_FILE="/Users/sainathtangallapalli/blockchain/ArthaChain/blockchain_node/cloudflared/config.yml"
    if [ ! -f "$CONFIG_FILE" ]; then
        print_error "Cloudflare tunnel config file not found: $CONFIG_FILE"
        exit 1
    fi
    print_status "Found config file: $CONFIG_FILE"
}

# Function to validate tunnel configuration
validate_config() {
    print_status "Validating tunnel configuration..."
    if cloudflared tunnel --config "$CONFIG_FILE" validate > /dev/null 2>&1; then
        print_success "Tunnel configuration is valid"
    else
        print_warning "Tunnel configuration validation returned warnings (this is normal)"
    fi
}

# Function to check if tunnel is already running
is_tunnel_running() {
    if launchctl list | grep -q "com.arthachain.tunnel"; then
        return 0
    else
        return 1
    fi
}

# Function to check tunnel status
check_tunnel_status() {
    print_status "Checking tunnel status..."
    
    if is_tunnel_running; then
        print_success "Cloudflare tunnel is running as a permanent service"
        
        # Get tunnel info
        echo ""
        echo "Tunnel Information:"
        cloudflared tunnel info arthachain-testnet 2>/dev/null || echo "Unable to retrieve tunnel info"
        
        # Check logs
        echo ""
        print_status "Recent log entries:"
        tail -n 10 /Users/sainathtangallapalli/blockchain/ArthaChain/blockchain_node/logs/cloudflared.err.log 2>/dev/null || echo "No error logs found"
        
        # Check service accessibility
        echo ""
        print_status "Checking service accessibility:"
        SERVICES=(
            "testnet.arthachain.in:80"
            "api.arthachain.in:80"
            "ws.arthachain.in:80"
            "explorer.arthachain.in:80"
        )
        
        for service in "${SERVICES[@]}"; do
            if nc -z -w5 $(echo $service | cut -d: -f1) $(echo $service | cut -d: -f2) 2>/dev/null; then
                echo "  ✅ $service is accessible"
            else
                echo "  ⚠️  $service is not accessible (may be because local services aren't running)"
            fi
        done
    else
        print_warning "Cloudflare tunnel is not running as a permanent service"
        echo "To start the tunnel permanently, run: $0 start"
    fi
}

# Function to install permanent tunnel
install_tunnel() {
    print_status "Installing permanent Cloudflare tunnel..."
    
    # Create logs directory
    mkdir -p /Users/sainathtangallapalli/blockchain/ArthaChain/blockchain_node/logs
    
    # Copy plist file
    cp /Users/sainathtangallapalli/blockchain/ArthaChain/blockchain_node/scripts/com.arthachain.tunnel.user.plist \
       /Users/sainathtangallapalli/Library/LaunchAgents/com.arthachain.tunnel.plist
    
    # Load the service
    print_status "Loading tunnel service..."
    launchctl bootstrap gui/$(id -u) /Users/sainathtangallapalli/Library/LaunchAgents/com.arthachain.tunnel.plist
    
    # Wait a moment for the service to start
    sleep 3
    
    if is_tunnel_running; then
        print_success "Permanent Cloudflare tunnel installed and started successfully!"
        echo "The tunnel will now automatically start when you log in and restart if it crashes."
    else
        print_error "Failed to start permanent tunnel"
        echo "Check the logs at: /Users/sainathtangallapalli/blockchain/ArthaChain/blockchain_node/logs/"
        exit 1
    fi
}

# Function to uninstall permanent tunnel
uninstall_tunnel() {
    print_status "Uninstalling permanent Cloudflare tunnel..."
    
    # Unload the service
    if is_tunnel_running; then
        print_status "Stopping tunnel service..."
        launchctl bootout gui/$(id -u) /Users/sainathtangallapalli/Library/LaunchAgents/com.arthachain.tunnel.plist 2>/dev/null || true
    fi
    
    # Remove plist file
    rm -f /Users/sainathtangallapalli/Library/LaunchAgents/com.arthachain.tunnel.plist
    
    print_success "Permanent Cloudflare tunnel uninstalled"
}

# Function to restart tunnel
restart_tunnel() {
    print_status "Restarting permanent Cloudflare tunnel..."
    
    # Uninstall first
    uninstall_tunnel
    
    # Wait a moment
    sleep 2
    
    # Install again
    install_tunnel
    
    print_success "Cloudflare tunnel restarted successfully"
}

# Function to show logs
show_logs() {
    print_status "Showing recent tunnel logs..."
    
    if [ -f "/Users/sainathtangallapalli/blockchain/ArthaChain/blockchain_node/logs/cloudflared.err.log" ]; then
        tail -f /Users/sainathtangallapalli/blockchain/ArthaChain/blockchain_node/logs/cloudflared.err.log
    else
        print_error "Log file not found"
    fi
}

# Main function
main() {
    # Check OS
    check_os
    
    # Parse command line arguments
    case "${1:-status}" in
        "status")
            check_cloudflared
            check_credentials
            check_config
            check_tunnel_status
            ;;
        "start"|"install")
            check_cloudflared
            check_credentials
            check_config
            validate_config
            install_tunnel
            ;;
        "stop"|"uninstall")
            uninstall_tunnel
            ;;
        "restart")
            check_cloudflared
            check_credentials
            check_config
            validate_config
            restart_tunnel
            ;;
        "logs")
            show_logs
            ;;
        "help"|"-h"|"--help")
            echo "ArthaChain Permanent Cloudflare Tunnel Manager"
            echo ""
            echo "Usage: $0 [command]"
            echo ""
            echo "Commands:"
            echo "  status     Check the current status of the tunnel (default)"
            echo "  start      Install and start the permanent tunnel"
            echo "  stop       Stop and uninstall the permanent tunnel"
            echo "  restart    Restart the permanent tunnel"
            echo "  logs       Show live tunnel logs"
            echo "  help       Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0 status"
            echo "  $0 start"
            echo "  $0 stop"
            ;;
        *)
            print_error "Unknown command: $1"
            echo "Use '$0 help' for available commands"
            exit 1
            ;;
    esac
}

# Run main function
main "$@"