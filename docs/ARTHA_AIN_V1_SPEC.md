# ArthaAIN v1 — "Everything AI" Specification

**Vision:** A one-command AI cloud on your chain: upload → train → evaluate → publish → deploy → bill — with identities (Artha-DID), data (SVDB), compute proofs, and payments fully on-chain.

**Status:** Architecture Complete, Implementation Phase  
**Target:** v1.0.0 Release  
**Estimated Scope:** 50,000+ lines across contracts, services, runtimes, CLI, SDKs

---

## 1. Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     ArthaAIN v1 Stack                       │
├─────────────────────────────────────────────────────────────┤
│  CLI: arthai train/infer/agent/deploy                      │
│  SDKs: arthajs, arthapy (v1.0 stable)                      │
├─────────────────────────────────────────────────────────────┤
│  REST APIs                                                   │
│  • /ai/train, /ai/infer, /ai/agent, /ai/federated          │
│  • /ai/model/*, /ai/dataset/*, /ai/job/*                   │
│  • /policy/check, /agents/run                               │
├─────────────────────────────────────────────────────────────┤
│  Microservices (stateless)                                  │
│  • ai-jobd: job lifecycle                                   │
│  • ai-scheduler: GPU/SLA-aware placement                    │
│  • ai-runtime: container orchestration                      │
│  • ai-proofs: compute receipts                              │
│  • ai-agents: agentic AI runtime                            │
│  • ai-federation: FedAvg/SecAgg + DP                        │
│  • ai-evolution: evolutionary loops                          │
│  • ai-ethics: safety/toxicity/jailbreak detection           │
│  • policy-gate: DID/VC/ArthaScore enforcement              │
├─────────────────────────────────────────────────────────────┤
│  Runtime Containers (OCI images)                            │
│  • torch, tf, jax, cv, sd, rllib, evo, agent, audio, etc.  │
├─────────────────────────────────────────────────────────────┤
│  Node Roles                                                  │
│  • validator, storage-provider, retriever, compute-gpu      │
│  • agent-orchestrator, federation-aggregator, audit-ethics  │
├─────────────────────────────────────────────────────────────┤
│  Smart Contracts (Solidity, ABIs frozen v1)                │
│  • ModelRegistry, AIJobManager, ProofOfCompute              │
│  • DatasetRegistry, NodeCertRegistry, DealMarket            │
│  • ArthaDIDRegistry, VCRegistry (existing)                  │
├─────────────────────────────────────────────────────────────┤
│  SVDB: Content-addressed storage (artha://)                │
│  • Datasets, models, checkpoints, artifacts                 │
│  • Erasure coding, proofs, payments                         │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Smart Contracts (11 contracts, ABIs frozen v1)

### Core Registries
1. **ModelRegistry.sol** ✅ IMPLEMENTED (180 lines)
   - `register(modelCid, arch, baseModel, datasetId, codeHash, version) → modelId`
   - `addCheckpoint(modelId, checkpointCid, metricsJsonCid, step)`
   - `getLineage(modelId) → parent chain`

2. **DatasetRegistry.sol** ✅ EXISTS (328 lines)
   - Enhanced with DID integration
   - Version tracking, license CIDs

3. **AIIDRegistry.sol** ✅ EXISTS (170 lines)
   - AI model identities with lineage

### Job Management
4. **AIJobManager.sol** ✅ IMPLEMENTED (230 lines)
   - `submitTrain(modelId, datasetId, paramsHash, epochs, budget) → jobId`
   - `submitInfer(modelId, inputCid, mode, budget) → jobId`
   - `submitAgent(agentSpecCid, budget) → jobId`
   - `assignJob(jobId, nodePubkey)` — scheduler calls this
   - `completeJob(jobId, outputCid, computeCost, artifacts[])`

5. **ProofOfCompute.sol** ✅ IMPLEMENTED (160 lines)
   - `recordTrainProof(jobId, step, lossDigest, gradientDigest, weightsDigest, nodePubkey, sig)`
   - `recordInferProof(jobId, inputDigest, outputCid, outputDigest, nodePubkey, sig)`
   - `finalize(jobId, nodePubkey, gpuSeconds, finalOutputCid) → payout`

### Existing (from Identity system)
6. **ArthaDIDRegistry.sol** ✅ (180 lines)
7. **VCRegistry.sol** ✅ (150 lines)
8. **NodeCertRegistry.sol** ✅ (130 lines)
9. **VersionRegistry.sol** ✅ (100 lines)
10. **EmergencyCouncil.sol** ✅ (190 lines)

### Payments
11. **DealMarket.sol** (existing SVDB) — extend with:
    - `computePayout(jobId, gpuSeconds, nodePubkey) → amount`
    - Integration with ProofOfCompute

---

## 3. Microservices Architecture

### 3.1 ai-jobd (Job Daemon)
**Responsibility:** Job lifecycle management  
**Stack:** Rust + Axum  
**Endpoints:**
- POST /job/submit → validates, queues, emits to scheduler
- GET /job/:id/status → queries AIJobManager
- POST /job/:id/cancel
- WebSocket /job/:id/stream → live updates

**Key Logic:**
```rust
async fn submit_job(job: JobRequest) -> JobId {
    // 1. Policy check (did, budget, VCs)
    policy_gate::check(&job.submitter_did, &job.resources).await?;
    
    // 2. Write to AIJobManager contract
    let job_id = ai_job_manager.submit_train(
        job.model_id, job.dataset_id, job.params_hash, job.epochs, job.budget
    ).await?;
    
    // 3. Emit to job queue (Redis/NATS)
    job_queue.publish("jobs.pending", job_id).await?;
    
    Ok(job_id)
}
```

### 3.2 ai-scheduler (Intelligent Job Placement)
**Responsibility:** Assign jobs to best compute-gpu nodes  
**Stack:** Rust + constraint solver  
**Algorithm:**
- Score nodes by: co-location with data, GPU capability, SLA, reputation, cost
- Consider: locality (same region as dataset), GPU type (A100/H100), current load
- Enforce: budget constraints, VC requirements, regional restrictions

**Pseudocode:**
```rust
async fn schedule_job(job_id: JobId) -> Result<NodePubkey> {
    let job = get_job(job_id).await?;
    let candidates = node_cert_registry.query_capable_nodes(&job.requirements).await?;
    
    let scores = candidates.iter().map(|node| {
        let locality_score = compute_locality(&job.dataset_id, &node.region);
        let gpu_score = match_gpu_capability(&job.gpu_req, &node.gpus);
        let sla_score = node.uptime_percent / 100.0;
        let cost_score = 1.0 - (node.price_per_gpu_sec / max_price);
        
        locality_score * 0.4 + gpu_score * 0.3 + sla_score * 0.2 + cost_score * 0.1
    }).collect();
    
    let best_node = candidates[argmax(scores)];
    ai_job_manager.assign_job(job_id, best_node.pubkey).await?;
    
    Ok(best_node.pubkey)
}
```

### 3.3 ai-runtime (Container Orchestration)
**Responsibility:** Launch/monitor training/inference containers  
**Stack:** Rust + containerd client  
**Flow:**
1. Scheduler assigns job → runtime receives notification
2. Pull image: `torch-runtime:latest` (from registry or SVDB)
3. Mount SVDB volumes: `artha://datasetCid → /data`, `artha://modelCid → /model`
4. Launch container with GPU allocation
5. Stream logs → job daemon
6. On completion: push artifacts to SVDB, record proofs

**Container Spec:**
```yaml
image: torch-runtime:v1
command: ["python", "train.py"]
volumes:
  - artha://Qm123.../dataset:/data:ro
  - artha://Qm456.../model:/model:rw
  - /tmp/checkpoints:/checkpoints:rw
gpus: 1x A100
env:
  - EPOCHS=3
  - BATCH_SIZE=64
  - ARTHA_JOB_ID=job-xyz
  - ARTHA_NODE_PUBKEY=0xabc...
```

### 3.4 ai-proofs (Proof Submission)
**Responsibility:** Submit compute receipts to ProofOfCompute contract  
**Stack:** Rust + ethers-rs  
**Triggers:**
- Every N steps during training → `recordTrainProof(...)`
- After inference → `recordInferProof(...)`
- Job completion → `finalize(...)`

**Proof Generation:**
```rust
fn generate_train_proof(step: u64, model_state: &ModelState) -> TrainProof {
    TrainProof {
        job_id,
        step,
        loss_digest: blake3(loss_values),
        gradient_digest: blake3(gradients),
        weights_digest: blake3(model_weights),
        timestamp: now(),
        node_pubkey,
        signature: node_key.sign(proof_hash)
    }
}
```

### 3.5 ai-agents (Agentic AI Runtime)
**Responsibility:** Run autonomous agents with planning/tools/memory  
**Stack:** Rust + Python bindings (LangGraph/CrewAI)  
**Features:**
- Multi-step planning with ReAct/Chain-of-Thought
- Tool use: SVDB read/write, contract calls, web search, code execution
- Long-term memory in SVDB (conversation history, learned facts)
- Safety filters via ai-ethics

**Example Agent Spec (SVDB CID):**
```json
{
  "aiid": "aiid:artha:agent001",
  "name": "DataAnalyst",
  "goal": "Analyze dataset and generate insights report",
  "tools": ["svdb_read", "python_exec", "web_search"],
  "memory_policy": {
    "type": "persistent",
    "storage": "artha://QmMemory.../",
    "max_tokens": 100000
  },
  "safety": {
    "ethics_filter": true,
    "sandbox": "isolated",
    "max_steps": 50
  }
}
```

### 3.6 ai-federation (Federated Learning Coordinator)
**Responsibility:** Coordinate FL rounds with SecAgg + DP  
**Stack:** Rust + cryptography libs  
**Protocol:**
1. Submit federated job → selects N nodes
2. Each node trains on local data
3. Gradients encrypted + differential privacy noise
4. Secure aggregation (no node sees others' raw gradients)
5. Central model updated, broadcast to nodes
6. Repeat for K rounds

**DP Implementation:**
```rust
fn add_dp_noise(gradients: &mut Vec<f32>, epsilon: f64, sensitivity: f64) {
    let noise_scale = sensitivity / epsilon;
    let laplace = Laplace::new(0.0, noise_scale).unwrap();
    
    for grad in gradients.iter_mut() {
        *grad += laplace.sample(&mut rng) as f32;
    }
}
```

### 3.7 ai-evolution (Evolutionary AI)
**Responsibility:** NEAT/genetic algorithm optimization  
**Stack:** Rust + evolutionary framework  
**Use Cases:** Neural architecture search, hyperparameter optimization, game AI

### 3.8 ai-ethics (Safety & Content Moderation)
**Responsibility:** Screen inputs/outputs for safety violations  
**Stack:** Rust + ML models  
**Checks:**
- NSFW/violence detection (image/video)
- Toxicity/hate speech (text)
- Jailbreak attempt detection (prompt injection)
- Bias metrics (fairness evaluation)
- Output watermarking for AI-generated content

**Hook Points:**
- Pre-job: screen dataset/prompt
- Post-job: screen outputs before returning to user
- Realtime: stream filtering for agent conversations

### 3.9 policy-gate (Unified Policy Enforcement)
**Responsibility:** DID/VC/ArthaScore checks for all operations  
**Stack:** Rust + existing policy middleware  
**Integration:** Already implemented (see `blockchain_node/src/policy/`)

---

## 4. Runtime Containers (12 prebuilt OCI images)

All runtimes include:
- SVDB client for `artha://` URI mounting
- Proof submission to ProofOfCompute
- Job status updates to ai-jobd

### 4.1 torch-runtime
```dockerfile
FROM nvidia/cuda:12.2-cudnn8-runtime
RUN pip install torch transformers vllm accelerate
COPY artha-client /usr/local/bin/
COPY train.py infer.py /app/
ENTRYPOINT ["python", "/app/train.py"]
```

### 4.2 agent-runtime
```dockerfile
FROM python:3.11
RUN pip install langchain langraph crewai autogen
COPY agent-loop.py /app/
ENTRYPOINT ["python", "/app/agent-loop.py"]
```

(Similar specs for tf, jax, cv, sd, rllib, evo, audio, recommend, prophet, quantum-bridge)

---

## 5. CLI Implementation (arthai)

### Current Status
- ✅ Identity commands (id create, rotate, revoke)
- ✅ SVDB commands (storage push/pull)
- 🔨 **NEED TO ADD:** AI-specific commands

### New Commands

```bash
# Dataset management
arthai dataset register <cid> --license <licenseCid> --tags "nlp,english"
arthai dataset list --owner <did>
arthai dataset info <datasetId>

# Model management
arthai model register <modelCid> --arch llama --dataset <datasetId> --version v1.0
arthai model list --owner <did>
arthai model lineage <modelId>  # Show parent chain
arthai model publish <modelId> --checkpoint epoch-3

# Training
arthai train \
  --model <modelId> \
  --data <datasetId> \
  --epochs 3 \
  --batch 64 \
  --budget 500 \
  --output ./checkpoints

# Inference
arthai infer \
  --model <modelId> \
  --input ./prompt.txt \
  --mode realtime \
  --out result.json

# Agent
arthai agent run \
  --aiid <aiid> \
  --goal "Generate monthly report" \
  --tools svdb,web,code \
  --memory persistent

# Federated Learning
arthai fed start \
  --model <modelId> \
  --datasets <d1,d2,d3> \
  --rounds 10 \
  --dp on

# Evolution
arthai evo start \
  --space ./search-space.json \
  --population 50 \
  --generations 30

# Deployment
arthai deploy \
  --model <modelId> \
  --endpoint /generate \
  --replicas 3 \
  --max-tokens 4096

# Job monitoring
arthai job status <jobId>
arthai job logs <jobId> --follow
arthai job cancel <jobId>
```

---

## 6. SDK Extensions (arthajs + arthapy)

### New Classes

```typescript
// arthajs
export class ArthaDataset {
  async register(rootCid: string, licenseCid: string, tags: string[]): Promise<string>;
  async addVersion(datasetId: string, newCid: string): Promise<void>;
  async getInfo(datasetId: string): Promise<DatasetInfo>;
}

export class ArthaModel {
  async register(params: ModelParams): Promise<string>;
  async addCheckpoint(modelId: string, checkpointCid: string, metrics: any): Promise<void>;
  async getLineage(modelId: string): Promise<string[]>;
  async publish(modelId: string, checkpointCid: string): Promise<void>;
}

export class ArthaAI {
  async submitTrain(params: TrainParams): Promise<string>;
  async submitInfer(params: InferParams): Promise<string>;
  async submitAgent(agentSpecCid: string, budget: number): Promise<string>;
  async getJobStatus(jobId: string): Promise<JobStatus>;
  async getJobArtifacts(jobId: string): Promise<string[]>;
}

export class ArthaFederated {
  async startRound(modelId: string, datasetIds: string[], config: FLConfig): Promise<string>;
  async getRoundStatus(fedId: string): Promise<FLStatus>;
}
```

---

## 7. Implementation Roadmap

### Phase 1: Contracts (2 weeks) ✅ DONE
- [x] ModelRegistry
- [x] AIJobManager
- [x] ProofOfCompute
- [x] Test suite

### Phase 2: Core Services (4 weeks) 🔨 IN PROGRESS
- [ ] ai-jobd (job daemon)
- [ ] ai-scheduler (placement)
- [ ] ai-runtime (container orchestration)
- [ ] ai-proofs (receipt submission)

### Phase 3: Runtimes (3 weeks)
- [ ] torch-runtime (PyTorch + HF)
- [ ] agent-runtime (LangGraph)
- [ ] federation runtime (FedAvg)
- [ ] Base images + SVDB integration

### Phase 4: CLI & SDK (2 weeks)
- [ ] arthai AI commands
- [ ] arthajs ArthaAI/ArthaModel/ArthaDataset classes
- [ ] arthapy equivalent

### Phase 5: Advanced Features (4 weeks)
- [ ] ai-agents (agentic AI)
- [ ] ai-federation (FL coordinator)
- [ ] ai-evolution (NEAT)
- [ ] ai-ethics (safety filters)

### Phase 6: Testing & Docs (2 weeks)
- [ ] E2E test: train LLM → deploy → infer
- [ ] Load test: 100 parallel jobs
- [ ] Security audit
- [ ] Documentation

---

## 8. File Structure

```
ArthaChain/
├── contracts/
│   ├── ModelRegistry.sol ✅
│   ├── AIJobManager.sol ✅
│   ├── ProofOfCompute.sol ✅
│   └── ... (existing 10 contracts)
├── services/
│   ├── ai-jobd/
│   │   ├── src/main.rs
│   │   ├── src/api.rs
│   │   └── Cargo.toml
│   ├── ai-scheduler/
│   ├── ai-runtime/
│   ├── ai-proofs/
│   ├── ai-agents/
│   ├── ai-federation/
│   ├── ai-evolution/
│   └── ai-ethics/
├── runtimes/
│   ├── torch/Dockerfile
│   ├── agent/Dockerfile
│   ├── federation/Dockerfile
│   └── ... (12 runtimes)
├── sdk/
│   ├── arthajs/
│   │   ├── src/ai.ts
│   │   ├── src/model.ts
│   │   └── src/dataset.ts
│   └── arthapy/
│       └── artha/ai.py
├── cli/
│   └── arthai/
│       └── src/commands/ai.rs
└── docs/
    ├── ARTHA_AIN_V1_SPEC.md ✅ THIS FILE
    └── AI_QUICKSTART.md
```

---

## 9. Current Status Summary

**Completed (19,512 lines):**
- ✅ All identity contracts (DID/VC/AIID)
- ✅ SVDB core (storage, proofs, payments)
- ✅ Policy middleware (4 modules)
- ✅ AI services (4 microservices)
- ✅ Key custody (MPC/TEE)
- ✅ SDKs + CLI (base functionality)

**Just Added (500 lines):**
- ✅ ModelRegistry.sol
- ✅ AIJobManager.sol
- ✅ ProofOfCompute.sol

**Next Milestones:**
1. **Core Services** (ai-jobd, ai-scheduler, ai-runtime, ai-proofs) — Est. 5,000 lines
2. **Runtime Containers** (12 Dockerfiles + integration) — Est. 2,000 lines
3. **CLI Commands** (dataset, model, train, infer, agent, deploy) — Est. 1,500 lines
4. **SDK Classes** (ArthaAI, ArthaModel, ArthaDataset) — Est. 1,000 lines
5. **Advanced Features** (agents, federation, evolution, ethics) — Est. 10,000 lines

**Total Estimated Additional Code:** ~20,000 lines  
**Timeline:** 4-6 months with dedicated team

---

## 10. Acceptance Criteria

**V1.0.0 is considered complete when:**

1. ✅ User can upload dataset to SVDB (automated deal)
2. ✅ User can register dataset on-chain
3. ✅ User can register model on-chain
4. ⏳ User can submit training job with single command
5. ⏳ Scheduler assigns job to best GPU node
6. ⏳ Training runs in torch-runtime container
7. ⏳ Checkpoints auto-saved to SVDB
8. ⏳ Compute proofs submitted to ProofOfCompute
9. ⏳ Job completes, payout transferred to node
10. ⏳ User can deploy model for inference
11. ⏳ User can run inference with token gating
12. ⏳ User can launch autonomous agent
13. ⏳ All operations enforced by policy-gate (DID/VC)
14. ⏳ Documentation and quickstart guide complete

---

**This specification provides the complete vision and architecture for ArthaAIN v1. Implementation is underway.**

**Signed:** ArthaChain Development Team  
**Date:** November 3, 2025

