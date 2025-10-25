#!/bin/bash

# ArthaChain VM Node Setup Script
# Sets up a complete ArthaChain node on a VM for community access

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

# Function to check if running as root
check_root() {
    if [ "$EUID" -ne 0 ]; then
        print_error "This script must be run as root"
        exit 1
    fi
}

# Function to update system
update_system() {
    print_status "Updating system packages..."
    
    if command -v apt >/dev/null 2>&1; then
        apt update && apt upgrade -y
    elif command -v yum >/dev/null 2>&1; then
        yum update -y
    elif command -v dnf >/dev/null 2>&1; then
        dnf update -y
    else
        print_warning "Unknown package manager, skipping system update"
    fi
    
    print_success "System updated"
}

# Function to install dependencies
install_dependencies() {
    print_status "Installing dependencies..."
    
    if command -v apt >/dev/null 2>&1; then
        apt install -y curl wget git build-essential pkg-config libssl-dev
    elif command -v yum >/dev/null 2>&1; then
        yum install -y curl wget git gcc make openssl-devel
    elif command -v dnf >/dev/null 2>&1; then
        dnf install -y curl wget git gcc make openssl-devel
    else
        print_error "No supported package manager found"
        exit 1
    fi
    
    print_success "Dependencies installed"
}

# Function to install Rust
install_rust() {
    print_status "Installing Rust..."
    
    # Check if Rust is already installed
    if command -v rustc >/dev/null 2>&1; then
        print_warning "Rust is already installed"
        return
    fi
    
    # Install Rust
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    
    # Source Rust environment
    source "$HOME/.cargo/env"
    
    print_success "Rust installed"
}

# Function to create arthachain user
create_user() {
    print_status "Creating arthachain user..."
    
    # Check if user already exists
    if id "arthachain" &>/dev/null; then
        print_warning "arthachain user already exists"
        return
    fi
    
    # Create user
    useradd -m -s /bin/bash arthachain
    usermod -aG sudo arthachain
    
    # Set password
    echo "arthachain:arthachain123" | chpasswd
    
    print_success "arthachain user created"
}

# Function to setup SSH access
setup_ssh() {
    print_status "Setting up SSH access for community members..."
    
    # Create .ssh directory for arthachain user
    mkdir -p /home/arthachain/.ssh
    chmod 700 /home/arthachain/.ssh
    chown arthachain:arthachain /home/arthachain/.ssh
    
    # Create authorized_keys file
    touch /home/arthachain/.ssh/authorized_keys
    chmod 600 /home/arthachain/.ssh/authorized_keys
    chown arthachain:arthachain /home/arthachain/.ssh/authorized_keys
    
    print_success "SSH setup completed"
    echo "Add community SSH keys to /home/arthachain/.ssh/authorized_keys"
}

# Function to install cloudflared
install_cloudflared() {
    print_status "Installing cloudflared..."
    
    if command -v cloudflared >/dev/null 2>&1; then
        print_warning "cloudflared is already installed"
        return
    fi
    
    # Install based on OS
    if command -v apt >/dev/null 2>&1; then
        # Ubuntu/Debian
        curl -L https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.deb -o cloudflared.deb
        dpkg -i cloudflared.deb
        rm cloudflared.deb
    elif command -v yum >/dev/null 2>&1; then
        # CentOS/RHEL
        curl -L https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.rpm -o cloudflared.rpm
        rpm -ivh cloudflared.rpm
        rm cloudflared.rpm
    else
        # Generic installation
        curl -L https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64 -o /usr/local/bin/cloudflared
        chmod +x /usr/local/bin/cloudflared
    fi
    
    print_success "cloudflared installed"
}

# Function to setup firewall
setup_firewall() {
    print_status "Setting up firewall rules..."
    
    # Open required ports
    if command -v ufw >/dev/null 2>&1; then
        # Ubuntu firewall
        ufw allow 22/tcp     # SSH
        ufw allow 30303/tcp  # P2P
        ufw allow 8080/tcp   # API
        ufw allow 8545/tcp   # RPC
        ufw allow 9184/tcp   # Metrics
        ufw allow 1900/tcp   # Main Node
        ufw --force enable
    elif command -v firewall-cmd >/dev/null 2>&1; then
        # CentOS/RHEL firewall
        firewall-cmd --permanent --add-port=22/tcp
        firewall-cmd --permanent --add-port=30303/tcp
        firewall-cmd --permanent --add-port=8080/tcp
        firewall-cmd --permanent --add-port=8545/tcp
        firewall-cmd --permanent --add-port=9184/tcp
        firewall-cmd --permanent --add-port=1900/tcp
        firewall-cmd --reload
    else
        print_warning "No supported firewall found, manual configuration required"
    fi
    
    print_success "Firewall configured"
}

# Function to create directory structure
create_directories() {
    print_status "Creating directory structure..."
    
    # Create directories
    mkdir -p /opt/arthachain/{data,config,logs,bin}
    mkdir -p /etc/cloudflared
    
    # Set ownership
    chown -R arthachain:arthachain /opt/arthachain
    chmod 755 /opt/arthachain
    
    print_success "Directories created"
}

# Function to create systemd service
create_systemd_service() {
    print_status "Creating systemd service..."
    
    # Create service file
    cat > /etc/systemd/system/arthachain-node.service << EOF
[Unit]
Description=ArthaChain Node
After=network.target

[Service]
Type=simple
User=arthachain
WorkingDirectory=/opt/arthachain
ExecStart=/opt/arthachain/bin/arthachain_node
Restart=always
RestartSec=10
Environment=RUST_LOG=info
Environment=ARTHACHAIN_DATA_DIR=/opt/arthachain/data
Environment=ARTHACHAIN_CONFIG_FILE=/opt/arthachain/config/node.yaml

[Install]
WantedBy=multi-user.target
EOF
    
    # Reload systemd
    systemctl daemon-reload
    
    print_success "Systemd service created"
}

# Function to create environment file
create_env_file() {
    print_status "Creating environment file..."
    
    # Create environment file
    cat > /opt/arthachain/.env << EOF
# ArthaChain Node Environment Variables
RUST_LOG=info
ARTHACHAIN_DATA_DIR=/opt/arthachain/data
ARTHACHAIN_CONFIG_FILE=/opt/arthachain/config/node.yaml
ARTHACHAIN_P2P_PORT=30303
ARTHACHAIN_API_PORT=8080
ARTHACHAIN_METRICS_PORT=9184
NODE_TYPE=fullnode
BOOTSTRAP_NODES=seed1.arthachain.in:30303,seed2.arthachain.in:30303
EXTERNAL_ADDRESS=$(curl -s ifconfig.me):30303
EOF
    
    # Set ownership
    chown arthachain:arthachain /opt/arthachain/.env
    chmod 600 /opt/arthachain/.env
    
    print_success "Environment file created"
}

# Function to show next steps
show_next_steps() {
    echo ""
    print_success "VM Node Setup Completed!"
    echo ""
    echo "Next steps:"
    echo "1. Add community SSH keys to /home/arthachain/.ssh/authorized_keys"
    echo "2. Configure node settings in /opt/arthachain/config/node.yaml"
    echo "3. Add Cloudflare credentials to /etc/cloudflared/"
    echo "4. Start the node with: sudo systemctl start arthachain-node"
    echo "5. Enable auto-start with: sudo systemctl enable arthachain-node"
    echo ""
    echo "Community members can connect via SSH:"
    echo "  ssh arthachain@YOUR_VM_IP"
    echo "  Password: arthachain123 (change this after first login)"
    echo ""
    echo "Node logs can be viewed with:"
    echo "  journalctl -u arthachain-node -f"
}

# Main function
main() {
    print_status "Starting ArthaChain VM Node Setup"
    
    # Check if running as root
    check_root
    
    # Update system
    update_system
    
    # Install dependencies
    install_dependencies
    
    # Install Rust
    install_rust
    
    # Create arthachain user
    create_user
    
    # Setup SSH access
    setup_ssh
    
    # Install cloudflared
    install_cloudflared
    
    # Setup firewall
    setup_firewall
    
    # Create directories
    create_directories
    
    # Create environment file
    create_env_file
    
    # Create systemd service
    create_systemd_service
    
    # Show next steps
    show_next_steps
    
    print_success "Setup completed successfully!"
}

# Run main function
main "$@"