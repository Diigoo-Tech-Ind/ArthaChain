#!/bin/bash

# ArthaChain Node User Data Script
# Following Sui/Aptos/Sei industry patterns

set -e

# Configuration
NODE_TYPE="${node_type}"
ENVIRONMENT="${environment}"
CHAIN_ID="201766"

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

# Update system
print_status "Updating system packages..."
apt-get update
apt-get upgrade -y

# Install required packages
print_status "Installing required packages..."
apt-get install -y \
    curl \
    wget \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    ca-certificates \
    unzip \
    jq \
    htop \
    nginx \
    prometheus-node-exporter

# Install Docker
print_status "Installing Docker..."
curl -fsSL https://get.docker.com -o get-docker.sh
sh get-docker.sh
usermod -aG docker ubuntu

# Install Docker Compose
print_status "Installing Docker Compose..."
curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
chmod +x /usr/local/bin/docker-compose

# Create ArthaChain user
print_status "Creating ArthaChain user..."
useradd -m -s /bin/bash arthachain
usermod -aG docker arthachain

# Create directories
print_status "Creating ArthaChain directories..."
mkdir -p /opt/arthachain/{bin,config,db,logs,scripts}
chown -R arthachain:arthachain /opt/arthachain

# Switch to arthachain user for setup
su - arthachain << 'EOF'

# Clone ArthaChain repository
print_status "Cloning ArthaChain repository..."
cd /opt/arthachain
git clone https://github.com/your-org/arthachain.git .

# Build Docker image
print_status "Building ArthaChain Docker image..."
docker build -f blockchain_node/Dockerfile -t arthachain-node:latest .

# Create node configuration
print_status "Creating node configuration..."
cat > /opt/arthachain/config/node.yaml << 'NODECONFIG'
# ArthaChain Node Configuration
node:
  type: "${NODE_TYPE}"
  moniker: "ArthaChain-${NODE_TYPE}-$(hostname)"
  chain_id: "${CHAIN_ID}"

network:
  api_address: "0.0.0.0:8080"
  p2p_address: "0.0.0.0:8084"
  metrics_address: "0.0.0.0:9184"
  external_address: "/dns/$(curl -s http://169.254.169.254/latest/meta-data/public-ipv4)/tcp/8084"

# Seed peers (will be updated after deployment)
seed_peers:
  - peer_id: "12D3KooWSeed1ArthaChainTestnet001"
    address: "/dns/seed1.arthachain.in/tcp/8084"
  - peer_id: "12D3KooWSeed2ArthaChainTestnet002"
    address: "/dns/seed2.arthachain.in/tcp/8084"
  - peer_id: "12D3KooWSeed3ArthaChainTestnet003"
    address: "/dns/seed3.arthachain.in/tcp/8084"

consensus:
  algorithm: "SVCP-SVBFT"
  min_validators: 3
  max_validators: 100

mining:
  enabled: true
  block_time: 5
  max_block_size: 10000

api:
  enable_transactions: true
  enable_blocks: true
  enable_consensus: true
  enable_network: true
  cors:
    allowed_origins: ["*"]
    allowed_methods: ["GET", "POST", "PUT", "DELETE"]
    allowed_headers: ["*"]

monitoring:
  prometheus: true
  health_checks: true
  log_level: "info"
NODECONFIG

# Create Docker Compose file
print_status "Creating Docker Compose configuration..."
cat > /opt/arthachain/docker-compose.yml << 'COMPOSECONFIG'
version: '3.8'

services:
  arthachain-node:
    image: arthachain-node:latest
    container_name: arthachain-${NODE_TYPE}-$(hostname)
    restart: unless-stopped
    ports:
      - "8080:8080"  # API port
      - "8084:8084"  # P2P port
      - "9184:9184"  # Metrics port
    volumes:
      - arthachain-data:/opt/arthachain/db
      - arthachain-config:/opt/arthachain/config
      - arthachain-logs:/opt/arthachain/logs
    environment:
      - RUST_LOG=info
      - RUST_BACKTRACE=1
      - NODE_TYPE=${NODE_TYPE}
      - CHAIN_ID=${CHAIN_ID}
    networks:
      - arthachain-network
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    labels:
      - "com.arthachain.service=node"
      - "com.arthachain.type=${NODE_TYPE}"
      - "com.arthachain.network=testnet"

  # Prometheus Node Exporter
  node-exporter:
    image: prom/node-exporter:latest
    container_name: node-exporter-$(hostname)
    restart: unless-stopped
    ports:
      - "9100:9100"
    volumes:
      - /proc:/host/proc:ro
      - /sys:/host/sys:ro
      - /:/rootfs:ro
    command:
      - '--path.procfs=/host/proc'
      - '--path.sysfs=/host/sys'
      - '--collector.filesystem.mount-points-exclude=^/(sys|proc|dev|host|etc)($$|/)'
    networks:
      - arthachain-network

volumes:
  arthachain-data:
    driver: local
  arthachain-config:
    driver: local
  arthachain-logs:
    driver: local

networks:
  arthachain-network:
    driver: bridge
COMPOSECONFIG

# Create startup script
print_status "Creating startup script..."
cat > /opt/arthachain/scripts/start_node.sh << 'STARTSCRIPT'
#!/bin/bash

# ArthaChain Node Startup Script

set -e

cd /opt/arthachain

print_status "Starting ArthaChain ${NODE_TYPE} node..."

# Start the node
docker-compose up -d

# Wait for node to be ready
print_status "Waiting for node to be ready..."
for i in {1..30}; do
    if curl -f http://localhost:8080/health > /dev/null 2>&1; then
        print_success "Node is ready!"
        break
    fi
    print_status "Attempt $i/30 - Waiting for node to start..."
    sleep 2
done

# Get node information
if curl -f http://localhost:8080/health > /dev/null 2>&1; then
    print_status "Node Information:"
    echo "  Type: ${NODE_TYPE}"
    echo "  API: http://$(curl -s http://169.254.169.254/latest/meta-data/public-ipv4):8080"
    echo "  P2P: $(curl -s http://169.254.169.254/latest/meta-data/public-ipv4):8084"
    echo "  Metrics: http://$(curl -s http://169.254.169.254/latest/meta-data/public-ipv4):9184"
    echo "  Health: http://$(curl -s http://169.254.169.254/latest/meta-data/public-ipv4):8080/health"
    
    # Register with load balancer (if applicable)
    print_status "Node startup complete!"
else
    print_error "Node failed to start properly"
    exit 1
fi
STARTSCRIPT

chmod +x /opt/arthachain/scripts/start_node.sh

# Create systemd service
print_status "Creating systemd service..."
sudo tee /etc/systemd/system/arthachain-node.service > /dev/null << 'SERVICECONFIG'
[Unit]
Description=ArthaChain Node Service
After=docker.service
Requires=docker.service

[Service]
Type=oneshot
RemainAfterExit=yes
User=arthachain
WorkingDirectory=/opt/arthachain
ExecStart=/opt/arthachain/scripts/start_node.sh
ExecStop=/usr/local/bin/docker-compose down
TimeoutStartSec=300

[Install]
WantedBy=multi-user.target
SERVICECONFIG

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable arthachain-node.service

EOF

# Start the ArthaChain node
print_status "Starting ArthaChain node service..."
systemctl start arthachain-node.service

# Configure Nginx as reverse proxy (optional)
print_status "Configuring Nginx reverse proxy..."
cat > /etc/nginx/sites-available/arthachain << 'NGINXCONFIG'
server {
    listen 80;
    server_name _;
    
    location / {
        proxy_pass http://localhost:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
    
    location /metrics {
        proxy_pass http://localhost:9184;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
NGINXCONFIG

ln -sf /etc/nginx/sites-available/arthachain /etc/nginx/sites-enabled/
rm -f /etc/nginx/sites-enabled/default
systemctl restart nginx

# Configure firewall
print_status "Configuring firewall..."
ufw --force enable
ufw allow ssh
ufw allow 80/tcp
ufw allow 443/tcp
ufw allow 8080/tcp
ufw allow 8084/tcp
ufw allow 9184/tcp
ufw allow 9100/tcp

# Install monitoring tools
print_status "Installing monitoring tools..."
apt-get install -y htop iotop nethogs

# Create monitoring script
print_status "Creating monitoring script..."
cat > /opt/arthachain/scripts/monitor.sh << 'MONITORSCRIPT'
#!/bin/bash

# ArthaChain Node Monitoring Script

echo "=== ArthaChain Node Status ==="
echo "Node Type: ${NODE_TYPE}"
echo "Environment: ${ENVIRONMENT}"
echo "Chain ID: ${CHAIN_ID}"
echo ""

echo "=== Docker Containers ==="
docker ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
echo ""

echo "=== Node Health ==="
if curl -f http://localhost:8080/health > /dev/null 2>&1; then
    echo "✅ Node is healthy"
    curl -s http://localhost:8080/health | jq .
else
    echo "❌ Node is unhealthy"
fi
echo ""

echo "=== System Resources ==="
echo "CPU Usage:"
top -bn1 | grep "Cpu(s)" | awk '{print $2}' | awk -F'%' '{print $1}'
echo "Memory Usage:"
free -h | grep Mem | awk '{print $3 "/" $2}'
echo "Disk Usage:"
df -h / | tail -1 | awk '{print $5}'
echo ""

echo "=== Network Connections ==="
netstat -tlnp | grep -E ':(8080|8084|9184)'
echo ""

echo "=== Logs (Last 10 lines) ==="
docker logs arthachain-${NODE_TYPE}-$(hostname) --tail 10
MONITORSCRIPT

chmod +x /opt/arthachain/scripts/monitor.sh

# Create log rotation
print_status "Configuring log rotation..."
cat > /etc/logrotate.d/arthachain << 'LOGROTATE'
/opt/arthachain/logs/*.log {
    daily
    missingok
    rotate 7
    compress
    delaycompress
    notifempty
    create 644 arthachain arthachain
    postrotate
        systemctl reload arthachain-node.service
    endscript
}
LOGROTATE

# Final setup
print_status "Finalizing setup..."
systemctl daemon-reload

# Print completion message
print_success "🎉 ArthaChain node setup complete!"
echo ""
print_status "Node Information:"
echo "  Type: ${NODE_TYPE}"
echo "  Environment: ${ENVIRONMENT}"
echo "  Chain ID: ${CHAIN_ID}"
echo "  API Port: 8080"
echo "  P2P Port: 8084"
echo "  Metrics Port: 9184"
echo ""
print_status "Useful Commands:"
echo "  Check status: systemctl status arthachain-node"
echo "  View logs: journalctl -u arthachain-node -f"
echo "  Monitor: /opt/arthachain/scripts/monitor.sh"
echo "  Restart: systemctl restart arthachain-node"
echo ""
print_success "🌐 Your ArthaChain node is now ready for the testnet!"
