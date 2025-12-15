# 🚀 Run Your ArthaChain Node

ArthaChain is designed to be run by ANYONE. You don't need a powerful server or technical expertise.

We have simplified everything into a **Single Universal Docker Image**.

---

## ⚡ Quick Start (Community Node)

The easiest way to join the network.

**Prerequisites:**
- [Docker Desktop](https://www.docker.com/products/docker-desktop/) installed
- [Git](https://git-scm.com/downloads) installed

**1. Clone the repository:**
```bash
git clone https://github.com/arthachain/arthachain.git
cd arthachain
```

**2. Start the node:**
```bash
# This uses the Universal Docker Setup
docker compose -f docker-compose.community.yml up -d
```

**That's it!** 🎉
Your node is now running and syncing with the testnet.

---

## 🔧 Managing Your Node

We provide simple commands to manage your node:

| Action | Command |
|--------|---------|
| **Start** | `docker compose -f docker-compose.community.yml up -d` |
| **Stop** | `docker compose -f docker-compose.community.yml down` |
| **Logs** | `docker compose -f docker-compose.community.yml logs -f` |
| **Status** | `docker compose -f docker-compose.community.yml ps` |

---

## 🤖 Node Roles (Advanced)

ArthaChain nodes can perform different roles. You can switch roles by setting the `NODE_ROLE` environment variable in `docker-compose.community.yml`.

### Available Roles:

| Role | Environment Variable | Description |
|------|---------------------|-------------|
| **Full Node** (Default) | `NODE_ROLE=full` | Validates blocks, syncs history. Good for general support. |
| **Validator** | `NODE_ROLE=validator` | Participates in consensus. Requires staking (coming soon). |
| **Miner** | `NODE_ROLE=miner` | Creates new blocks. Requires CPU/GPU power. |
| **P2P Relay** | `NODE_ROLE=p2p-relay` | Helps network connectivity. Low resource usage. |
| **AI Worker** | `NODE_ROLE=ai-worker` | Processes AI compute jobs. Requires GPU. |
| **API Gateway** | `NODE_ROLE=api-gateway` | High-performance API endpoint for dApps. |

### How to Change Role:
Edit `docker-compose.community.yml`:
```yaml
    environment:
      - NODE_ROLE=validator  # Change this value
```
Then restart:
```bash
docker compose -f docker-compose.community.yml up -d
```

---

## 🔌 Standard Ports

We use standard ports for simplicity:

- **8080**: HTTP API (REST)
- **8545**: EVM RPC (MetaMask)
- **8084**: P2P Networking
- **9184**: Metrics (Prometheus)

---

## ❓ FAQ

**Q: Do I need to build from source?**
A: No! The Docker setup builds everything automatically or pulls the image.

**Q: Can I run multiple nodes?**
A: Yes, but you'll need to change the ports in `docker-compose.community.yml` to avoid conflicts.

**Q: Does it support Apple Silicon (M1/M2/M3)?**
A: Yes, the Docker build works on ARM64 architectures.

---

## 🛠️ Advanced: Building Specific Images

If you prefer building specific, lightweight images for a single role, we provide dedicated Dockerfiles:

```bash
# Build Validator
docker build -f docker/validator/Dockerfile -t arthachain/validator .

# Build RPC Node
docker build -f docker/rpc/Dockerfile -t arthachain/rpc .

# Build AI Worker
docker build -f docker/ai/Dockerfile -t arthachain/ai .
```

Available roles in `docker/`: `validator`, `rpc`, `storage`, `p2p`, `ai`, `sentry`, `api-gateway`, `grpc`, `websocket`, `metrics`.
