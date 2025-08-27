# 🚀 ArthaChain API Hosting Guide

## 📊 **API Overview**

**Total APIs Ready: 93 Endpoints + 8 WebSocket Events**

### **🎯 Core Statistics**
- **REST Endpoints**: 85+
- **WebSocket Events**: 8 real-time event types
- **Target Performance**: 100,000+ TPS
- **Block Time**: 3 seconds
- **Chain ID**: 201766 (ArthaChain Mainnet)

---

## 🐳 **Quick Start with Docker**

### **Option 1: Docker Compose (Recommended)**

```bash
# Clone and navigate to project
cd ArthaChain/blockchain_node

# Start production environment
docker-compose up -d

# Check logs
docker-compose logs -f arthachain-node

# Health check
curl http://localhost:8080/health
```

### **Option 2: Docker Build**

```bash
# Build image
docker build -t arthachain-node .

# Run container
docker run -d \
  --name arthachain-mainnet \
  -p 8080:8080 \
  -p 8545:8545 \
  -p 9944:9944 \
  -p 30303:30303 \
  -v arthachain_data:/app/data \
  arthachain-node
```

---

## 🖥️ **Manual Installation**

### **Prerequisites**
- **RAM**: Minimum 4GB, Recommended 8GB+
- **Storage**: Minimum 50GB SSD
- **OS**: Ubuntu 20.04+ / Debian 11+ / RHEL 8+
- **Rust**: Latest stable version

### **Installation Steps**

```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 2. Clone repository
git clone https://github.com/your-org/ArthaChain.git
cd ArthaChain/blockchain_node

# 3. Build project
cargo build --release

# 4. Run production script
./scripts/start_production.sh
```

---

## 🌐 **Production Deployment (arthavhain.in)**

### **Domain Configuration**

1. **Main API**: `https://arthavhain.in` → Port 8080
2. **JSON-RPC**: `https://rpc.arthavhain.in` → Port 8545  
3. **WebSocket**: `wss://ws.arthavhain.in` → Port 9944
4. **P2P Network**: `tcp://p2p.arthavhain.in` → Port 30303

### **Nginx/Traefik Configuration**

```nginx
# Nginx example for arthavhain.in
server {
    listen 443 ssl http2;
    server_name arthavhain.in;
    
    # SSL certificates
    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;
    
    # API endpoints
    location / {
        proxy_pass http://localhost:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
    
    # WebSocket upgrade
    location /api/v1/ws {
        proxy_pass http://localhost:9944;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
    }
}

# JSON-RPC subdomain
server {
    listen 443 ssl http2;
    server_name rpc.arthavhain.in;
    
    location / {
        proxy_pass http://localhost:8545;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

---

## 📡 **API Endpoints Reference**

### **🔗 Core Blockchain APIs (20)**
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/blocks/latest` | Latest block |
| GET | `/api/v1/blocks/:hash` | Block by hash |
| GET | `/api/v1/transactions/:hash` | Transaction details |
| POST | `/api/v1/transactions` | Submit transaction |
| GET | `/api/v1/accounts/:address` | Account info |

### **🏛️ Consensus & Validators (15)**
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/consensus/validators` | Active validators |
| GET | `/api/v1/consensus/status` | Consensus status |
| POST | `/api/consensus/vote` | Submit vote |
| POST | `/api/consensus/propose` | Submit proposal |

### **🚰 Faucet System (5)**
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/testnet/faucet/request` | Request tokens |
| GET | `/api/v1/testnet/faucet/status` | Faucet status |
| GET | `/faucet` | Faucet dashboard |

### **⛽ Gas-Free Applications (7)**
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/gas-free` | Dashboard |
| POST | `/api/v1/testnet/gas-free/register` | Register app |
| POST | `/api/v1/testnet/gas-free/check` | Check eligibility |
| GET | `/api/v1/testnet/gas-free/apps` | List apps |

### **💼 Wallet Integration (6)**
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/wallet` | Connect page |
| GET | `/api/v1/wallet/supported` | Supported wallets |
| POST | `/api/wallet/rpc` | Wallet RPC |

### **🔌 WebSocket Events (8)**
- `new_block` - Real-time block updates
- `new_transaction` - Transaction broadcasts
- `transaction_confirmed` - Confirmations
- `mempool_update` - Mempool changes
- `consensus_update` - Consensus state
- `chain_reorg` - Reorganizations
- `validator_update` - Validator changes
- `network_status` - Network health

---

## 🔧 **Configuration**

### **Environment Variables**
```bash
# Core settings
export ARTHACHAIN_ENV=production
export CHAIN_ID=201766
export API_HOST=0.0.0.0
export API_PORT=8080

# Performance
export MAX_TPS=100000
export BLOCK_TIME=3000
export THREAD_POOL_SIZE=32

# Features
export ENABLE_SHARDING=true
export ENABLE_AI_OPTIMIZATION=true
export ENABLE_FRAUD_DETECTION=true
```

### **Production Config File**
```toml
# /app/config/production.toml
[node]
chain_id = 201766
network_name = "arthachain_mainnet"

[api]
host = "0.0.0.0"
port = 8080
cors_origins = ["*"]

[performance]
target_tps = 100000
enable_ai_optimization = true

[security]
enable_fraud_detection = true
enable_quantum_resistance = true
```

---

## 📊 **Monitoring & Health Checks**

### **Health Check Endpoints**
- **Main Health**: `GET /health`
- **API Status**: `GET /status`
- **Metrics**: `GET /metrics`
- **Network Stats**: `GET /api/v1/network/stats`

### **Monitoring Setup**
```bash
# Prometheus metrics
curl http://localhost:9092/metrics

# Health check
curl http://localhost:8080/health

# Network status
curl http://localhost:8080/api/v1/network/status
```

---

## 🚀 **Performance Optimization**

### **System Tuning**
```bash
# Increase file limits
echo "* soft nofile 65536" >> /etc/security/limits.conf
echo "* hard nofile 65536" >> /etc/security/limits.conf

# Network optimization
echo 'net.core.rmem_max = 134217728' >> /etc/sysctl.conf
echo 'net.core.wmem_max = 134217728' >> /etc/sysctl.conf
sysctl -p
```

### **Resource Allocation**
- **CPU**: 4+ cores (8+ recommended)
- **RAM**: 8GB minimum (16GB+ recommended)
- **Storage**: NVMe SSD preferred
- **Network**: 1Gbps+ bandwidth

---

## 🔒 **Security**

### **Firewall Configuration**
```bash
# UFW example
ufw allow 22/tcp      # SSH
ufw allow 80/tcp      # HTTP
ufw allow 443/tcp     # HTTPS
ufw allow 8080/tcp    # API
ufw allow 8545/tcp    # RPC
ufw allow 9944/tcp    # WebSocket
ufw allow 30303/tcp   # P2P
ufw enable
```

### **SSL/TLS Setup**
```bash
# Let's Encrypt with Certbot
certbot --nginx -d arthavhain.in -d rpc.arthavhain.in -d ws.arthavhain.in
```

---

## 🐛 **Troubleshooting**

### **Common Issues**

**Port Already in Use**
```bash
# Check what's using the port
lsof -i :8080
# Kill the process
sudo kill -9 <PID>
```

**Memory Issues**
```bash
# Check memory usage
free -h
# Monitor node memory
ps aux | grep arthachain
```

**Connection Issues**
```bash
# Check network connectivity
curl -I http://localhost:8080/health
# Check logs
docker logs arthachain-mainnet
```

### **Logs Location**
- **Docker**: `docker logs arthachain-mainnet`
- **Manual**: `./logs/arthachain.log`
- **System**: `/var/log/arthachain/`

---

## 📞 **Support & Resources**

### **API Documentation**
- **Main Docs**: `https://arthavhain.in/docs`
- **OpenAPI Spec**: `https://arthavhain.in/api/v1/openapi.json`
- **Postman Collection**: Available on request

### **Community**
- **GitHub**: Repository issues and discussions
- **Discord**: Real-time community support
- **Telegram**: Official announcements

---

## ✅ **Final Checklist**

- [ ] System requirements met (4GB+ RAM, 50GB+ storage)
- [ ] Docker/Docker Compose installed
- [ ] Domain DNS configured (arthavhain.in)
- [ ] SSL certificates installed
- [ ] Firewall configured
- [ ] Health checks passing
- [ ] Monitoring setup
- [ ] Backup strategy in place

**🎉 Your ArthaChain APIs are ready to HOST at arthavhain.in!**
