#!/bin/bash

# ArthaChain Decentralized Node Setup Script
# Sets up a Cloudflare tunnel for a decentralized node

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

# Function to detect node IP
detect_node_ip() {
    # Try multiple methods to detect IP
    local ip=""
    
    # Method 1: Use hostname -I (Linux)
    if command -v hostname >/dev/null 2>&1; then
        ip=$(hostname -I 2>/dev/null | awk '{print $1}' | head -n 1)
        if [ -n "$ip" ] && [ "$ip" != "127.0.0.1" ]; then
            echo "$ip"
            return
        fi
    fi
    
    # Method 2: Use ifconfig (macOS/Linux)
    if command -v ifconfig >/dev/null 2>&1; then
        # Try to get non-local IP
        ip=$(ifconfig 2>/dev/null | grep 'inet ' | grep -v '127.0.0.1' | awk '{print $2}' | head -n 1)
        if [ -n "$ip" ]; then
            echo "$ip"
            return
        fi
    fi
    
    # Method 3: Use ip command (Linux)
    if command -v ip >/dev/null 2>&1; then
        ip=$(ip route get 1.1.1.1 2>/dev/null | awk '{for(i=1;i<=NF;i++) if($i=="src") print $(i+1); exit}')
        if [ -n "$ip" ]; then
            echo "$ip"
            return
        fi
    fi
    
    # Fallback to localhost
    echo "127.0.0.1"
}

# Function to create node-specific configuration
create_node_config() {
    local node_ip=$1
    local node_name=$2
    local config_dir=$3
    
    print_status "Creating configuration for node: $node_name"
    print_status "Node IP: $node_ip"
    
    # Create config directory
    mkdir -p "$config_dir"
    
    # Create the configuration file
    cat > "$config_dir/config.yml" << EOF
# ArthaChain Cloudflare Tunnel Configuration for $node_name
# Node IP: $node_ip

tunnel: arthachain-$node_name
credentials-file: $config_dir/credentials.json

# Ingress rules for routing traffic
ingress:
  # Main Blockchain Node
  - hostname: testnet.arthachain.in
    service: http://$node_ip:1900
    originRequest:
      connectTimeout: 30s
      readTimeout: 30s
      writeTimeout: 30s
      noTLSVerify: false
    headers:
      - name: X-Forwarded-For
        value: "{{ .ClientIP }}"
      - name: X-Real-IP
        value: "{{ .ClientIP }}"
      - name: X-Forwarded-Proto
        value: "https"

  # Global API Service
  - hostname: api.arthachain.in
    service: http://$node_ip:1910
    originRequest:
      connectTimeout: 30s
      readTimeout: 30s
      writeTimeout: 30s
      noTLSVerify: false
    headers:
      - name: X-Forwarded-For
        value: "{{ .ClientIP }}"
      - name: X-Real-IP
        value: "{{ .ClientIP }}"
      - name: X-Forwarded-Proto
        value: "https"

  # Real-Time API Server
  - hostname: ws.arthachain.in
    service: http://$node_ip:1920
    originRequest:
      connectTimeout: 30s
      readTimeout: 30s
      writeTimeout: 30s
      noTLSVerify: false
    headers:
      - name: X-Forwarded-For
        value: "{{ .ClientIP }}"
      - name: X-Real-IP
        value: "{{ .ClientIP }}"
      - name: X-Forwarded-Proto
        value: "https"

  # Block Explorer API
  - hostname: explorer.arthachain.in
    service: http://$node_ip:1930
    originRequest:
      connectTimeout: 30s
      readTimeout: 30s
      writeTimeout: 30s
      noTLSVerify: false
    headers:
      - name: X-Forwarded-For
        value: "{{ .ClientIP }}"
      - name: X-Real-IP
        value: "{{ .ClientIP }}"
      - name: X-Forwarded-Proto
        value: "https"

  # Documentation Server
  - hostname: docs.arthachain.in
    service: http://$node_ip:1940
    originRequest:
      connectTimeout: 30s
      readTimeout: 30s
      writeTimeout: 30s
      noTLSVerify: false
    headers:
      - name: X-Forwarded-For
        value: "{{ .ClientIP }}"
      - name: X-Real-IP
        value: "{{ .ClientIP }}"
      - name: X-Forwarded-Proto
        value: "https"

  # Metrics Server
  - hostname: metrics.arthachain.in
    service: http://$node_ip:1950
    originRequest:
      connectTimeout: 30s
      readTimeout: 30s
      writeTimeout: 30s
      noTLSVerify: false
    headers:
      - name: X-Forwarded-For
        value: "{{ .ClientIP }}"
      - name: X-Real-IP
        value: "{{ .ClientIP }}"
      - name: X-Forwarded-Proto
        value: "https"

  # Faucet Service
  - hostname: faucet.arthachain.in
    service: http://$node_ip:1960
    originRequest:
      connectTimeout: 30s
      readTimeout: 30s
      writeTimeout: 30s
      noTLSVerify: false
    headers:
      - name: X-Forwarded-For
        value: "{{ .ClientIP }}"
      - name: X-Real-IP
        value: "{{ .ClientIP }}"
      - name: X-Forwarded-Proto
        value: "https"

  # RPC Service
  - hostname: rpc.arthachain.in
    service: http://$node_ip:1970
    originRequest:
      connectTimeout: 30s
      readTimeout: 30s
      writeTimeout: 30s
      noTLSVerify: false
    headers:
      - name: X-Forwarded-For
        value: "{{ .ClientIP }}"
      - name: X-Real-IP
        value: "{{ .ClientIP }}"
      - name: X-Forwarded-Proto
        value: "https"

  # Catch-all rule for unmatched hostnames
  - service: http_status:404
EOF

    print_success "Configuration created at $config_dir/config.yml"
}

# Function to show usage
show_usage() {
    echo "ArthaChain Decentralized Node Setup"
    echo ""
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -i, --ip IP_ADDRESS     Node IP address (default: auto-detected)"
    echo "  -n, --name NODE_NAME    Node name/identifier (default: hostname)"
    echo "  -d, --dir CONFIG_DIR    Configuration directory (default: ./cloudflared)"
    echo "  -h, --help              Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0"
    echo "  $0 -i 192.168.1.100 -n validator-1"
    echo "  $0 -n fullnode-1 -d /etc/cloudflared"
}

# Main function
main() {
    local node_ip=""
    local node_name=""
    local config_dir="./cloudflared"
    
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
            -d|--dir)
                config_dir="$2"
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
        node_ip=$(detect_node_ip)
        if [ "$node_ip" = "127.0.0.1" ]; then
            print_warning "Could not detect external IP address. Using 127.0.0.1"
            print_warning "For multi-node deployment, please specify the IP address explicitly."
        fi
    fi
    
    # Auto-detect node name if not provided
    if [ -z "$node_name" ]; then
        node_name=$(hostname)
    fi
    
    # Create node configuration
    create_node_config "$node_ip" "$node_name" "$config_dir"
    
    echo ""
    print_success "Node setup completed!"
    echo "To create a tunnel for this node in Cloudflare:"
    echo "  1. Go to Cloudflare Zero Trust Dashboard"
    echo "  2. Navigate to Access → Tunnels"
    echo "  3. Create a new tunnel named 'arthachain-$node_name'"
    echo "  4. Download the credentials file and place it at $config_dir/credentials.json"
    echo "  5. Run the tunnel with: cloudflared tunnel --config $config_dir/config.yml run"
    echo ""
    echo "For permanent setup, create a systemd service or launch agent with this configuration."
}

# Run main function
main "$@"