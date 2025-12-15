# ⚠ DEPRECATED DOCKER SETUP

The separate Docker configurations in this directory have been **DEPRECATED** and replaced by a **SINGLE UNIVERSAL IMAGE**.

## New Setup

Use the `docker/Dockerfile` at the root of the `docker` directory.

### Why?
We simplified the node setup so you don't need to choose between 10 different Docker files. One image handles everything:
- Validator
- Miner
- RPC Node
- Storage Node
- P2P Relay
- AI Job Worker

### Usage

Use the provided `docker-compose.community.yml` in the project root:
```bash
docker compose -f docker-compose.community.yml up -d
```

Or build manually:
```bash
docker build -t arthachain/node -f docker/Dockerfile .
```

### Configuration
Set `NODE_ROLE` env var to switch modes:
- `validator`
- `full` (default)
- `rpc`
- `ai-worker`
