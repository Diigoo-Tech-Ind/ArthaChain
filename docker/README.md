# ArthaChain Docker Deployment Guide

## Overview

ArthaChain uses a **unified binary architecture** where all nodes run the same `arthachain_node` binary. The key difference is:

- **Normal Node**: Full participation - mining/validation/sharding roles are **automatically selected by the system** based on stake, network topology, and consensus round robin.
- **Sentry Node**: Acts as a protective shield for validators (public-facing gateway).
- **RPC Node**: Read-only JSON-RPC service for dApps and wallets (no consensus participation).

---

## Quick Start

### Start a Normal Node (Full Participation)
```bash
# Single node
docker compose up validator-node -d

# Or just run directly
docker run -d \
  --name arthachain-node \
  -p 1900:1900 \
  -p 8084:8084 \
  -p 9184:9184 \
  -v arthachain-data:/app/data \
  arthachain/validator-node:latest
```

The node will automatically:
- Join the P2P network
- Sync blockchain state
- Participate in consensus (based on stake)
- Be assigned to shards (based on network topology)
- Produce blocks (when selected as leader)

---

### Start a Sentry Node (Gateway/Shield)
```bash
docker compose up sentry-node -d

# Direct run with validator connection
docker run -d \
  --name arthachain-sentry \
  -p 1900:1900 \
  -p 8084:8084 \
  -e VALIDATOR_NODES=internal-validator:8084 \
  arthachain/sentry-node:latest
```

Sentry nodes:
- Accept public P2P connections
- Filter malicious traffic  
- Relay valid messages to internal validators
- Hide validator IP addresses from public exposure

---

### Start an RPC Node (Read-Only)
```bash
docker compose up rpc-service -d

# Direct run
docker run -d \
  --name arthachain-rpc \
  -p 8545:8545 \
  -p 1900:1900 \
  -p 8084:8084 \
  arthachain/rpc-service:latest
```

RPC nodes provide:
- Ethereum-compatible JSON-RPC (port 8545)
- ArthaChain REST API (port 1900)
- Read-only blockchain queries
- High availability for dApps and wallets

---

## Production Deployment Topology

```
┌─────────────────────────────────────────────────────────────────┐
│                        PUBLIC INTERNET                          │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                               ▼
┌──────────────────────────────────────────────────────────────────┐
│                      SENTRY NODES (Public)                        │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐           │
│  │ sentry-1    │    │ sentry-2    │    │ sentry-3    │           │
│  │ Port: 8084  │    │ Port: 8084  │    │ Port: 8084  │           │
│  └──────┬──────┘    └──────┬──────┘    └──────┬──────┘           │
└─────────┼──────────────────┼──────────────────┼──────────────────┘
          │                  │                  │
          └──────────────────┼──────────────────┘
                             │ (Private Network)
                             ▼
┌──────────────────────────────────────────────────────────────────┐
│                    NORMAL NODES (Internal)                        │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐           │
│  │ node-1      │    │ node-2      │    │ node-3      │           │
│  │ (validator) │    │ (miner)     │    │ (shard-1)   │           │
│  │ Auto-role   │    │ Auto-role   │    │ Auto-role   │           │
│  └─────────────┘    └─────────────┘    └─────────────┘           │
└──────────────────────────────────────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────────┐
│                      RPC NODES (Public API)                       │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐           │ 
│  │ rpc-1       │    │ rpc-2       │    │ rpc-3       │           │
│  │ Port: 8545  │    │ Port: 8545  │    │ Port: 8545  │           │
│  └─────────────┘    └─────────────┘    └─────────────┘           │
└──────────────────────────────────────────────────────────────────┘
```

---

## Port Reference

| Port | Protocol | Service | Description |
|------|----------|---------|-------------|
| **1900** | HTTP | ArthaChain API | Native REST API |
| **8084** | TCP/UDP | P2P | libp2p peer connections |
| **8545** | HTTP | EVM RPC | Ethereum JSON-RPC |
| **8546** | WebSocket | WS RPC | Real-time subscriptions |
| **9184** | HTTP | Metrics | Prometheus metrics |
| **9090** | HTTP | Prometheus | Monitoring UI |
| **9944** | HTTP/2 | gRPC | gRPC API |
| **30303** | TCP/UDP | P2P Alt | Alternative P2P port |

---

## Docker Compose Commands

```bash
# Start all services
docker compose up -d

# Start specific node types
docker compose up validator-node -d      # Normal node
docker compose up sentry-node -d         # Sentry node
docker compose up rpc-service -d         # RPC node

# View logs
docker compose logs -f validator-node

# Stop all services
docker compose down

# Rebuild images
docker compose build
```

---

## Environment Variables

### Normal Node
```yaml
environment:
  - RUST_LOG=info           # Logging level
  - API_PORT=1900           # REST API port
  - P2P_PORT=8084           # P2P port
  - METRICS_PORT=9184       # Prometheus metrics port
```

### Sentry Node
```yaml
environment:
  - RUST_LOG=info
  - API_PORT=1900
  - P2P_PORT=8084
  - VALIDATOR_NODES=validator-1:8084,validator-2:8084  # Internal validators
```

### RPC Node
```yaml
environment:
  - RUST_LOG=info
  - API_PORT=1900
  - P2P_PORT=8084
  - SYNC_FROM_PEERS=true    # Sync blockchain from peers
```

---

## Kubernetes Deployment

For EKS deployment, each Dockerfile can be used with the following pattern:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: arthachain-validator
spec:
  replicas: 3
  selector:
    matchLabels:
      app: arthachain
      role: validator
  template:
    spec:
      containers:
      - name: arthachain-node
        image: {ECR_URL}/arthachain/validator-node:latest
        ports:
        - containerPort: 1900
        - containerPort: 8084
        - containerPort: 9184
        env:
        - name: RUST_LOG
          value: "info"
```

---

## Dockerfile Summary

| Service | Dockerfile | Purpose |
|---------|-----------|---------|
| **validator-node** | `docker/validator-node/Dockerfile` | Normal node - auto role selection |
| **rpc-service** | `docker/rpc-service/Dockerfile` | Read-only RPC |
| **sentry-node** | `docker/sentry-node/Dockerfile` | Gateway/shield |
| **api-gateway** | `docker/api-gateway/Dockerfile` | REST API gateway |
| **websocket-rpc** | `docker/websocket-rpc/Dockerfile` | Real-time events |
| **p2p-network** | `docker/p2p-network/Dockerfile` | Peer discovery |
| **storage-node** | `docker/storage-node/Dockerfile` | Data persistence |
| **grpc-server** | `docker/grpc-server/Dockerfile` | gRPC API |
| **prometheus-metrics** | `docker/prometheus-metrics/Dockerfile` | Monitoring |

---

*ArthaChain - Next-generation blockchain with AI-native features*
