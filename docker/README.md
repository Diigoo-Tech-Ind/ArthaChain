# ArthaChain Docker Deployment Guide

## Overview

ArthaChain uses a microservices-based Docker architecture where each component has its own optimized Dockerfile. This allows for fine-grained control over builds and deployments.

---

## 🏗 Dockerfile Map

| Service | Dockerfile | Purpose |
|---------|-----------|---------|
| **validator-node** | `docker/validator/Dockerfile` | Full node for consensus & mining |
| **rpc-service** | `docker/rpc/Dockerfile` | Read-only JSON-RPC for dApps |
| **sentry-node** | `docker/sentry/Dockerfile` | Public gateway for DDoS protection |
| **api-gateway** | `docker/api-gateway/Dockerfile` | REST API gateway |
| **websocket-rpc** | `docker/websocket/Dockerfile` | Real-time event streaming |
| **p2p-network** | `docker/p2p/Dockerfile` | P2P discovery and routing |
| **storage-node** | `docker/storage/Dockerfile` | SVDB data node |
| **grpc-server** | `docker/grpc/Dockerfile` | High-performance gRPC API |
| **ai-worker** | `docker/ai/Dockerfile` | AI model training & inference |
| **metrics** | `docker/metrics/Dockerfile` | Prometheus metrics exporter |

---

## 🚀 Quick Start

### Start All Services
```bash
cd docker
docker compose up -d
```

### Start Specific Service
```bash
docker compose up -d validator-node
```

### Build Specific Image
```bash
docker build -t arthachain/rpc -f docker/rpc/Dockerfile .
```

---

## ⚙️ Configuration

Each service is configured via environment variables in `docker-compose.yml`.

### Common Variables
- `NODE_ROLE`: Defines the behavior (validator, miner, etc.)
- `RUST_LOG`: Logging level (info, debug, trace)
- `DATA_DIR`: Path to data volume

---

## ☸️ Kubernetes

Use the specific Dockerfile for each Deployment kind.

```yaml
kind: Deployment
metadata:
  name: rpc-node
spec:
  template:
    spec:
      containers:
      - name: rpc
        image: arthachain/rpc-service:latest
        ports:
        - containerPort: 8545
```
