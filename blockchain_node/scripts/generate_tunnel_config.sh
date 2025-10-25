#!/bin/bash

# ArthaChain Cloudflare Tunnel Configuration Generator
# Generates node-specific tunnel configurations for decentralized deployment

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

# Function to get local IP address
get_local_ip() {
    # Try to get the primary local IP address
    if command -v ip >/dev/null 2>&1; then
        ip route get 1.1.1.1 2>/dev/null | awk '{for(i=1;i<=NF;i++) if($i=="src") print $(i+1); exit}'
    elif command -v route >/dev/null 2>&1; then
        route get default 2>/dev/null | grep interface | awk '{print $2}' | xargs ifconfig 2>/dev/null | grep 'inet ' | grep -v 127.0.0.1 | awk '{print $2}' | head -n 1
    else
        echo "127.0.0.1"
    fi
}

# Function to validate IP address
validate_ip() {
    local ip=$1
    if [[ $ip =~ ^[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}$ ]]; then
        IFS='.' read -ra ADDR <<< "$ip"
        for i in "${ADDR[@]}"; do
            if [[ $i -gt 255 ]]; then
                return 1
            fi
        done
        return 0
    else
        return 1
    fi
}

# Function to generate configuration
generate_config() {
    local node_ip=$1
    local output_file=$2
    local node_name=$3
    
    print_status "Generating Cloudflare tunnel configuration for node: $node_name"
    print_status "Node IP Address: $node_ip"
    
    # Create the directory if it doesn't exist
    mkdir -p "$(dirname "$output_file")"
    
    # Generate the configuration by replacing NODE_IP in the template
    sed "s/NODE_IP/$node_ip/g" /Users/sainathtangallapalli/blockchain/ArthaChain/blockchain_node/cloudflared/config_template.yml > "$output_file"
    
    # Update the tunnel name to include node identifier
    sed -i '' "s/arthachain-testnet/arthachain-$node_name/g" "$output_file" 2>/dev/null || sed -i "s/arthachain-testnet/arthachain-$node_name/g" "$output_file"
    
    print_success "Configuration generated successfully: $output_file"
    
    # Show the generated configuration
    echo ""
    print_status "Generated configuration:"
    echo "---"
    cat "$output_file"
    echo "---"
}

# Function to setup credentials
setup_credentials() {
    local credentials_source=$1
    local credentials_dest=$2
    
    if [ ! -f "$credentials_source" ]; then
        print_error "Source credentials file not found: $credentials_source"
        echo "Please ensure the Cloudflare credentials file exists."
        exit 1
    fi
    
    # Create the directory if it doesn't exist
    mkdir -p "$(dirname "$credentials_dest")"
    
    # Copy the credentials file
    cp "$credentials_source" "$credentials_dest"
    
    # Set proper permissions
    chmod 600 "$credentials_dest"
    
    print_success "Credentials file copied to: $credentials_dest"
}

# Function to show usage
show_usage() {
    echo "ArthaChain Cloudflare Tunnel Configuration Generator"
    echo ""
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -i, --ip IP_ADDRESS     Node IP address (default: auto-detected)"
    echo "  -n, --name NODE_NAME    Node name/identifier (default: hostname)"
    echo "  -o, --output FILE       Output configuration file (default: ./config.yml)"
    echo "  -c, --credentials FILE  Source credentials file (default: ~/.cloudflared/*.json)"
    echo "  -d, --dest-credentials FILE  Destination credentials file (default: /etc/cloudflared/*.json)"
    echo "  -h, --help              Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0"
    echo "  $0 -i 192.168.1.100 -n validator-1"
    echo "  $0 -i 10.0.0.5 -n fullnode-1 -o /etc/cloudflared/config.yml"
}

# Main function
main() {
    local node_ip=""
    local node_name=""
    local output_file="./config.yml"
    local credentials_source=""
    local credentials_dest="/etc/cloudflared/cc3311b0-2a5c-4444-aacc-668576634499.json"
    
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -i|--ip)
                node_ip="$2"
                shift 2
                ;;
            -n|--name)
                node_name="$2"
                shift 2
                ;;
            -o|--output)
                output_file="$2"
                shift 2
                ;;
            -c|--credentials)
                credentials_source="$2"
                shift 2
                ;;
            -d|--dest-credentials)
                credentials_dest="$2"
                shift 2
                ;;
            -h|--help)
                show_usage
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done
    
    # Auto-detect IP if not provided
    if [ -z "$node_ip" ]; then
        node_ip=$(get_local_ip)
        if [ "$node_ip" = "127.0.0.1" ]; then
            print_warning "Could not detect local IP address. Using 127.0.0.1"
            print_warning "For multi-node deployment, please specify the IP address explicitly."
        fi
    fi
    
    # Validate IP address
    if ! validate_ip "$node_ip"; then
        print_error "Invalid IP address: $node_ip"
        exit 1
    fi
    
    # Auto-detect node name if not provided
    if [ -z "$node_name" ]; then
        node_name=$(hostname)
    fi
    
    # Auto-detect credentials source if not provided
    if [ -z "$credentials_source" ]; then
        # Try to find the credentials file
        if [ -d "/Users/sainathtangallapalli/.cloudflared" ]; then
            credentials_source=$(ls /Users/sainathtangallapalli/.cloudflared/*.json 2>/dev/null | head -n 1)
        fi
        
        if [ -z "$credentials_source" ]; then
            print_error "Could not find Cloudflare credentials file automatically."
            echo "Please specify the credentials file with -c option."
            exit 1
        fi
    fi
    
    # Generate the configuration
    generate_config "$node_ip" "$output_file" "$node_name"
    
    # Setup credentials
    setup_credentials "$credentials_source" "$credentials_dest"
    
    echo ""
    print_success "Configuration generation completed!"
    echo "To start the tunnel on this node, run:"
    echo "  sudo cloudflared tunnel --config $output_file run arthachain-$node_name"
    echo ""
    echo "For permanent setup, create a systemd service or launch agent with this configuration."
}

# Run main function
main "$@"