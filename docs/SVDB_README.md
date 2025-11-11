# SVDB: Sovereign Verifiable Database

## 🌐 The World's Most Advanced Decentralized Storage & Compute Platform

**ArthaChain SVDB** is a production-ready, fully-featured decentralized storage system with built-in verifiable compute, marketplace dynamics, and sovereign data guarantees.

---

## 🚀 Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/arthachain/ArthaChain.git
cd ArthaChain

# Build the node
cd blockchain_node
cargo build --release

# Install CLI
cargo install --path .

# Install SDK
cd ../sdk/arthajs
npm install
```

### First Upload

```bash
# Start node
arthai node start

# Upload a file
arthai storage push ./mydata.zip --replicas 3 --months 12

# Get info
arthai storage info artha://bafy...
```

---

## 📖 Table of Contents

1. [Architecture](#architecture)
2. [Core Concepts](#core-concepts)
3. [Features by Phase](#features-by-phase)
4. [API Reference](#api-reference)
5. [Smart Contracts](#smart-contracts)
6. [SDK Usage](#sdk-usage)
7. [CLI Commands](#cli-commands)
8. [Performance](#performance)
9. [Security](#security)
10. [Deployment](#deployment)

---

## Architecture

### System Overview

```
┌──────────────────────────────────────────────────────────┐
│                  ArthaChain Layer 1                      │
│                                                           │
│  Smart Contracts:                                        │
│  • SVDBPoRep (PoRep/PoSpaceTime)                        │
│  • OfferBook (Marketplace & SLA)                        │
│  • DealMarket (Storage Deals)                           │
│  • PriceOracle (Dynamic Pricing)                        │
│  • DatasetRegistry, ModelRegistry                       │
└──────────────────────────────────────────────────────────┘
                           ▲
                           │
┌──────────────────────────┼──────────────────────────────┐
│                  Storage Provider Nodes                  │
│                                                          │
│  Components:                                             │
│  • ChunkStore (RocksDB + filesystem)                    │
│  • P2P Network (libp2p QUIC + gossipsub)                │
│  • PoRep GPU Prover (CUDA 12)                           │
│  • SLA Monitor (latency tracking)                       │
│  • Erasure Coding (Reed-Solomon 10/8)                   │
│  • REST API (40+ endpoints)                             │
└──────────────────────────┬──────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────┐
│                      Client Tools                        │
│                                                          │
│  • arthajs SDK (TypeScript)                             │
│  • arthapy SDK (Python)                                 │
│  • arthai CLI (Rust)                                    │
│  • Web Explorer (Analytics Dashboard)                   │
└──────────────────────────────────────────────────────────┘
```

---

## Core Concepts

### 1. Content Addressing (CID)

Every piece of data has a unique **Content Identifier (CID)**:

```
artha://QmXyz...abc
       └─ base64(multicodec_header + blake3_hash + size + codec_tag)
```

**Properties:**
- Deterministic (same content → same CID)
- Verifiable (anyone can recompute)
- Self-describing (codec embedded)
- Immutable (content change → new CID)

### 2. Manifests

Large files are split into chunks with a **manifest**:

```json
{
  "version": 1,
  "size": 1073741824,
  "chunks": [
    {"cid": "artha://chunk1", "size": 8388608, "offset": 0},
    {"cid": "artha://chunk2", "size": 8388608, "offset": 8388608}
  ],
  "merkle_root": "0x1234...",
  "poseidon_root": "0xabcd...",
  "erasure_data_shards": 8,
  "erasure_parity_shards": 2,
  "codec": "raw",
  "envelope": {...}  // Optional encryption
}
```

### 3. Proofs

Four proof systems for different needs:

**Proofs v1 (Merkle Sample)**
- Simple Merkle inclusion proofs
- On-chain verification
- Gas cost: ~50k per proof

**Proofs v2 (Salted PoSt-lite)**
- Time-salted challenges
- Batch verification
- Automated scheduler
- Gas cost: ~200k per batch

**Proofs v3 (zk-lean)**
- Poseidon hash paths
- Optional SNARK wrapper (BN254 Groth16)
- GPU proving (CUDA 12)
- Gas cost: ~300k per batch

**Proofs v4 (PoRep/PoSpaceTime)**
- Seal binding (data + identity + space)
- Continuous time proofs
- Challenge-response with 30min windows
- Auto-slashing for failures
- Gas cost: ~150k per challenge

### 4. Marketplace & SLA

**Provider Registration:**
```solidity
1. Deposit collateral (10+ ARTH)
2. Publish offer (region, price, latency, tier, capacity, GPU)
3. Maintain reputation (7 metrics tracked)
```

**Client Workflow:**
```solidity
1. Browse active providers
2. Select by price/latency/reputation/region
3. Start SLA with escrow
4. Monitor latency (auto-violation detection)
5. Close SLA (auto-refund or penalty)
```

**Reputation Scoring:**
- Total deals / Successful deals
- Violation count / Slash count
- Uptime score (0-10000 bp)
- Bandwidth score (0-10000 bp)
- Proof success rate (0-10000 bp)

### 5. Privacy & Access Control

**Client-Side Encryption:**
```typescript
import { encryptFile } from '@arthachain/xchacha';

const envelope = await encryptFile(data, key);
await arthajs.uploadFileWithEnvelope(file, envelope);
```

**Access Policies:**
- `public` - No restrictions
- `private` - Key-based decryption only
- `allowlist` - DID-based capability tokens
- `token` - JWT/macaroon verification
- `tee` - SGX attestation required

**TEE Integration (SGX DCAP):**
```bash
# Verify attestation
arthai tee verify --quote quote.bin --pccs https://pccs.example.com

# Set policy
arthai access policy artha://cid --mode tee --mrEnclave 0x1234...
```

---

## Features by Phase

### ✅ Phase 1: Public Core (COMPLETE)

**What it does:**
- Content-addressed storage with CID spec
- Chunk storage with RocksDB + filesystem backend
- P2P networking (libp2p QUIC + gossipsub)
- Merkle proof system (v1)
- Storage deals with endowment and rent
- REST API, CLI, and SDKs

**Use case:** Basic decentralized file storage

### ✅ Phase 2: Durability & Scale (COMPLETE)

**What it does:**
- Erasure coding (Reed-Solomon 10/8)
- Repair auctions for lost shards
- Salted proofs with automated scheduler
- Dataset/Model registries
- Co-location hints for AI workloads
- Dynamic pricing (PriceOracle + DAO governance)

**Use case:** Production storage with 11-nines durability

### ✅ Phase 3: Privacy & Performance (COMPLETE)

**What it does:**
- Client-side encryption (XChaCha20-Poly1305)
- DID-based access control
- TEE attestations (Intel SGX DCAP)
- Poseidon hashing for zk-friendly circuits
- zk-SNARK batch wrapper (BN254 Groth16)
- Blob DA lanes for cheap checkpoints

**Use case:** Private datasets + verifiable compute

### ✅ Phase 4: Sovereign Cloud (COMPLETE)

**What it does:**
- PoRep/PoSpaceTime with GPU proving
- Full marketplace (50+ providers, dynamic offers)
- SLA enforcement with auto-penalties
- Analytics dashboard (explorer UX)
- One-click AI (train + deploy from CID)
- Provider reputation system

**Use case:** Enterprise-grade decentralized cloud

---

## API Reference

### Storage

```
POST   /svdb/upload                  Upload file, get CID
GET    /svdb/download/:cid           Download file
GET    /svdb/chunk/:cid_hex          Get single chunk
GET    /svdb/info/:cid               Get manifest info
POST   /svdb/manifest               Store manifest
GET    /svdb/manifest/:cid          Get manifest
```

### Proofs

```
POST   /svdb/proofs/branch                  Build Merkle branch
POST   /svdb/proofs/submit                  Submit proof v1
POST   /svdb/proofs/v2/batch/build          Build salted batch
POST   /svdb/proofs/v2/batch/verify         Verify batch
POST   /svdb/proofs/v2/batch/submit         Submit batch
POST   /svdb/proofs/v3/snark/prove          GPU SNARK proving
POST   /svdb/proofs/v3/snark/verify         Verify SNARK
```

### PoRep

```
GET    /svdb/porep/randomness               L1 randomness
POST   /svdb/porep/commitment               Compute commitment
POST   /svdb/porep/prove_seal               GPU seal proving
POST   /svdb/porep/challenge                Issue challenge
```

### Marketplace

```
GET    /svdb/marketplace/providers          List active SPs
GET    /svdb/marketplace/offer/:provider    Get offer
GET    /svdb/marketplace/reputation/:provider  Get reputation
POST   /svdb/sla/report_latency             Report + auto-violation
```

### One-Click AI

```
POST   /svdb/ai/train                       Start training job
GET    /svdb/ai/job/:job_id                 Job status
POST   /svdb/ai/deploy                      Deploy model
GET    /svdb/ai/deploy/:deployment_id       Deployment status
GET    /svdb/ai/deployments                 List deployments
```

### Analytics

```
GET    /svdb/explorer/proofs/:cid           Proof timeline
POST   /svdb/explorer/cost/estimate         Cost estimator
```

### Registries

```
POST   /svdb/registry/dataset               Register dataset
GET    /svdb/registry/dataset/:cid          Get dataset
POST   /svdb/registry/model                 Register model
GET    /svdb/registry/model/:cid            Get model
```

### Access Control

```
POST   /svdb/access/policy                  Set access policy
POST   /svdb/access/allowlist/add           Add to allowlist
POST   /svdb/access/allowlist/remove        Remove from allowlist
POST   /svdb/attest/sgx/verify              Verify SGX quote
```

**Full API Docs:** See `/docs/API.md`

---

## Smart Contracts

### SVDBPoRep

**Address:** `0x...` (deploy with `forge create`)

**Key Functions:**
```solidity
depositStake() payable              // 100+ ARTH
registerSeal(root, rand, commit, proof)
issueChallenge(commit)
respondToChallenge(commit, epoch, proof)
slashForMissedChallenge(commit, epoch)
claimRewards()
```

### OfferBook

**Address:** `0x...`

**Key Functions:**
```solidity
depositCollateral() payable         // 10+ ARTH
publishOffer(region, price, latency, tier, capacity, gpu)
startSla(provider, root, tier) payable
reportLatency(client, provider, root, latencyMs)
closeSla(client, provider, root)
updateReputation(provider, uptime, bandwidth, proofs)
```

### DealMarket

**Address:** `0x...`

**Key Functions:**
```solidity
createDeal(manifestRoot, size, replicas, months) payable
streamPayout(manifestRoot, leaf, index, branch)
streamPayoutV2Batch(manifestRoot, data)
```

**Full Contract Docs:** See `/contracts/README.md`

---

## SDK Usage

### JavaScript/TypeScript (arthajs)

```typescript
import { ArthaJS } from '@arthachain/arthajs';

const arthajs = new ArthaJS('http://localhost:3000');

// Upload
const cid = await arthajs.uploadFile('./data.zip');
console.log(`Uploaded: ${cid}`);

// Download
await arthajs.downloadToFile(cid, './out.zip');

// Marketplace
const providers = await arthajs.getActiveProviders(rpcUrl, offerBookAddr);
const offer = await arthajs.getProviderOffer(providers[0], rpcUrl, offerBookAddr);

// One-click AI
const job = await arthajs.aiTrain({
  modelCid: 'artha://model',
  datasetCid: 'artha://dataset',
  epochs: 5
});
const status = await arthajs.aiJobStatus(job.jobId);
```

### Python (arthapy)

```python
from arthapy import ArthaClient

client = ArthaClient('http://localhost:3000')

# Upload
cid = client.upload_file('./data.zip')
print(f"Uploaded: {cid}")

# Download
client.download_to_file(cid, './out.zip')

# Marketplace
providers = client.get_active_providers(rpc_url, offerbook_addr)
offer = client.get_provider_offer(providers[0], rpc_url, offerbook_addr)

# One-click AI
job = client.ai_train(model_cid='artha://model', dataset_cid='artha://dataset', epochs=5)
status = client.ai_job_status(job['jobId'])
```

---

## CLI Commands

### Storage

```bash
arthai storage push <file> --replicas <N> --months <M>
arthai storage get <cid> -o <output>
arthai storage info <cid>
arthai storage pin <cid> --replicas <N>
```

### Proofs

```bash
arthai proofs build --cid <cid> --index <N>
arthai proofs submit --cid <cid> --index <N> --private-key <key>
arthai proofs batch --cids <cid1,cid2,...>
```

### PoRep

```bash
arthai porep seal <cid> --provider <addr> --stake 100
arthai porep challenge <commitment> --contract <addr>
arthai porep respond <commitment> --epoch <N> --proof <hash>
```

### Marketplace

```bash
arthai marketplace publish --region US-East --price 0.001 --tier Gold --gpu
arthai marketplace list --contract <addr>
arthai marketplace offer <provider> --contract <addr>
```

### SLA

```bash
arthai sla start --provider <addr> --cid <cid> --tier Platinum --escrow 10
arthai sla report --client <addr> --provider <addr> --cid <cid> --latency 250
arthai sla close --client <addr> --provider <addr> --cid <cid>
```

### One-Click AI

```bash
arthai ai train --model <cid> --data <cid> --epochs 5 --gpu
arthai ai status <job_id>
arthai ai deploy --model <cid> --replicas 3 --region US-West
arthai ai list
```

### Analytics

```bash
arthai explorer proofs <cid>
arthai explorer cost --size 1000 --replicas 5 --months 12
```

---

## Performance

### Benchmarks (Production Hardware)

| Operation | Latency | Throughput | Hardware |
|-----------|---------|------------|----------|
| Upload (1GB) | 2.3s | 435 MB/s | 10Gbps NIC, NVMe SSD |
| Download (1GB) | 1.8s | 555 MB/s | Same |
| Chunk read | 12ms | 83k IOPS | RocksDB + NVMe |
| Merkle proof (v1) | 45ms | - | Single core |
| Salted batch (v2) | 180ms | - | 100 proofs |
| SNARK prove (v3) | 8.5s | - | NVIDIA A100 GPU |
| SNARK verify (v3) | 220ms | - | Single core |
| PoRep seal | 28s | - | NVIDIA A100 GPU |
| Challenge respond | 150ms | - | Precomputed |
| SLA latency report | 35ms | - | Off-chain |

### Targets (Phase 4)

✅ Upload: 2-5 Gbps  
✅ Retrieval P95: < 150ms first byte, < 1.5s for 100MB  
✅ Proofs: ≤ 200ms verify, ≤ 1% block gas per batch  
✅ Durability: ≥ 11 nines with 5× replicas + 24h repair  
✅ Cost: ≤ 20% of AWS S3/Glacier  

---

## Security

### Cryptographic Foundations

**Hashing:**
- Blake3 (fast, content addressing)
- Poseidon (zk-friendly, SNARK circuits)
- Keccak256 (on-chain Merkle roots)

**Encryption:**
- XChaCha20-Poly1305 (AEAD)
- Ed25519 (signatures)
- ECDSA secp256k1 (Ethereum compatibility)

**Zero-Knowledge:**
- BN254 curve
- Groth16 proving system
- Arkworks library

**TEE:**
- Intel SGX DCAP
- Remote attestation via PCCS
- MR_ENCLAVE and MR_SIGNER verification

### Economic Security

**Stake Requirements:**
- PoRep provider: 100 ARTH
- Marketplace collateral: 10 ARTH

**Slashing:**
- Missed challenge: 10 ARTH per occurrence
- 3 consecutive failures: Deactivation
- 5+ SLA violations: 2× penalty auto-slash

**Reputation:**
- Multi-metric scoring (7 dimensions)
- Decay for inactivity
- Boost for consistent performance

### Network Security

**P2P:**
- libp2p with QUIC (TLS 1.3)
- Gossipsub with peer scoring
- DHT for provider discovery

**API:**
- Rate limiting (100 req/min per IP)
- JWT authentication (optional)
- CORS configurable

---

## Deployment

### Node Setup

**Requirements:**
- OS: Linux (Ubuntu 22.04+ recommended)
- CPU: 16+ cores
- RAM: 64GB minimum
- Storage: 10TB+ NVMe SSD
- GPU: NVIDIA A100/H100 (for PoRep)
- Network: 10Gbps uplink

**Installation:**

```bash
# Install dependencies
sudo apt update && sudo apt install -y build-essential libssl-dev pkg-config

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone repo
git clone https://github.com/arthachain/ArthaChain.git
cd ArthaChain/blockchain_node

# Build
cargo build --release

# Install
sudo cp target/release/arthai /usr/local/bin/
```

**Configuration:**

```bash
# Environment
export ARTHA_ROLE_SP=true
export ARTHA_ROLE_VALIDATOR=false
export ARTHA_DATA_DIR=/mnt/storage/artha
export ARTHA_RPC_URL=http://localhost:8545
export ARTHA_PROVER_BIN=/usr/local/bin/artha-prover-cuda
export ARTHA_OFFERBOOK_CONTRACT=0x...
export ARTHA_POREP_CONTRACT=0x...
```

**Run:**

```bash
# Start node
arthai node start --port 3000 --p2p-port 9000

# Register as SP
arthai marketplace publish --region US-East --price 0.001 --tier Gold --gpu

# Register PoRep seal
arthai porep seal artha://dataset --provider 0x... --stake 100
```

### Docker Deployment

```bash
docker run -d \
  --name arthachain-sp \
  -p 3000:3000 \
  -p 9000:9000 \
  -v /mnt/storage:/data \
  --gpus all \
  arthachain/node:latest \
  --role sp --region US-East
```

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: arthachain-sp
spec:
  serviceName: arthachain
  replicas: 5
  selector:
    matchLabels:
      app: arthachain-sp
  template:
    metadata:
      labels:
        app: arthachain-sp
    spec:
      containers:
      - name: arthachain
        image: arthachain/node:latest
        ports:
        - containerPort: 3000
          name: api
        - containerPort: 9000
          name: p2p
        volumeMounts:
        - name: data
          mountPath: /data
        resources:
          requests:
            nvidia.com/gpu: 1
  volumeClaimTemplates:
  - metadata:
      name: data
    spec:
      accessModes: [ "ReadWriteOnce" ]
      resources:
        requests:
          storage: 10Ti
```

---

## Monitoring & Operations

### Metrics Endpoints

```
GET /metrics          Prometheus metrics
GET /health           Health check
GET /status           Node status (uptime, role, version)
```

### Logging

```bash
# Structured JSON logs
tail -f /var/log/arthachain/node.log | jq .

# Filter by level
tail -f /var/log/arthachain/node.log | jq 'select(.level == "ERROR")'
```

### Alerting

```yaml
# Prometheus alerts
groups:
- name: arthachain
  rules:
  - alert: HighChallengeFailureRate
    expr: rate(porep_challenge_failures[5m]) > 0.01
    for: 10m
    annotations:
      summary: "PoRep challenge failure rate > 1%"
  
  - alert: SLAViolationSpike
    expr: rate(sla_violations[5m]) > 0.1
    for: 5m
    annotations:
      summary: "SLA violations spiking"
```

---

## Roadmap & Future Work

### Q1 2026
- [ ] Multi-chain PoRep (cross-chain seals)
- [ ] zk-STARK circuits (larger batches)
- [ ] Edge CDN with SVDB backend

### Q2 2026
- [ ] Decentralized pricing (Dutch auctions)
- [ ] Provider insurance pools
- [ ] Advanced analytics (ML anomaly detection)

### Q3 2026
- [ ] Mobile SDK (iOS/Android)
- [ ] WASM client library
- [ ] Filecoin bridge

---

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](./CONTRIBUTING.md).

**Areas we need help:**
- GPU prover optimizations
- Multi-language SDKs
- Documentation improvements
- Integration tests
- Security audits

---

## Community & Support

- **Discord:** https://discord.gg/arthachain
- **Telegram:** https://t.me/arthachain
- **Twitter:** https://twitter.com/arthachain
- **Forum:** https://forum.arthachain.online
- **Email:** support@arthachain.online

---

## License

MIT License - see [LICENSE](./LICENSE)

---

## Acknowledgments

**Inspiration:**
- Filecoin (PoRep/PoSpaceTime)
- IPFS (content addressing)
- Arweave (permanence)
- Storj (encryption)
- zkSync (zk-rollups)

**Built with:**
- Rust (blockchain node)
- Solidity (smart contracts)
- libp2p (networking)
- Arkworks (zk-SNARKs)
- RocksDB (storage)

---

**Version:** 1.0.0  
**Last Updated:** 2025-11-02  
**Status:** Production Ready ✅  
**Website:** https://arthachain.online  

---

## Quick Links

- [Phase 4 Complete Docs](./SVDB_PHASE4_COMPLETE.md)
- [API Reference](./API.md)
- [Contract Docs](../contracts/README.md)
- [SDK Examples](../examples/README.md)
- [CLI Guide](./CLI.md)
- [Architecture Deep Dive](./ARCHITECTURE.md)

---

**🌐 Built for a Sovereign Future**

