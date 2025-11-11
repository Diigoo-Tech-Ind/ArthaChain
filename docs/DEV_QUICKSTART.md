# ArthaAIN v1 — Developer Quickstart Guide

Get up and running with ArthaAIN in **5 minutes**.

## Prerequisites

- Rust toolchain (stable)
- Docker & Docker Compose
- Node.js 18+ (for SDK)
- Python 3.11+ (for SDK)

## 1. Setup Identity (30 seconds)

```bash
# Create your DID
arthai id create

# Output: did:artha:abc123...
```

## 2. Upload Data (1 minute)

```bash
# Upload dataset to SVDB (auto-deal, returns CID)
arthai storage push ./my-dataset --replicas 5 --months 12

# Output: artha://bafyBeibazData123...
```

## 3. Register Dataset & Model (1 minute)

```bash
# Register dataset
arthai dataset register artha://bafyBeibazData123 \
    --license artha://bafyLicense \
    --tags "medical,imaging"

# Output: ds:artha:xyz456

# Register model
arthai model register artha://bafyModelInit \
    --arch llama \
    --dataset ds:artha:xyz456 \
    --version v1

# Output: mid:789
```

## 4. Train Model (2 minutes)

```bash
# Submit training job (automated scheduling + proofs + payouts)
arthai train \
    --model mid:789 \
    --data ds:artha:xyz456 \
    --epochs 3 \
    --batch 64 \
    --budget 600

# Output: job-abc123
# Status: Running...
# ✅ Job completed!
```

## 5. Deploy Model (30 seconds)

```bash
# Deploy for inference
arthai deploy \
    --model mid:789 \
    --endpoint /generate \
    --replicas 2 \
    --max-tokens 2048

# Output: deploy-xyz789
# Endpoint: https://ain.artha.online/mid:789/v1
```

## 6. Run Inference (30 seconds)

```bash
# Infer from deployed model
arthai infer \
    --model mid:789 \
    --input "Write a 2-line poem about ArthaChain" \
    --out response.json

# ✅ Output saved to response.json
```

## Complete Example: End-to-End Training → Deployment

```bash
#!/bin/bash
set -e

# 1. Identity
echo "📝 Creating DID..."
DID=$(arthai id create | grep -oP 'did:artha:\S+')

# 2. Upload dataset
echo "📦 Uploading dataset..."
DATASET_CID=$(arthai storage push ./dataset --replicas 5 --months 12 | grep -oP 'artha://\S+')
DATASET_ID=$(arthai dataset register "$DATASET_CID" --license artha://bafyLicense | grep -oP 'ds:artha:\S+')

# 3. Register model
echo "🧠 Registering model..."
MODEL_ID=$(arthai model register artha://bafyModel --arch llama --dataset "$DATASET_ID" | grep -oP 'mid:\S+')

# 4. Train
echo "🚀 Training model..."
JOB_ID=$(arthai train --model "$MODEL_ID" --data "$DATASET_ID" --epochs 3 --batch 64 --budget 600 | grep -oP 'job-\S+')
echo "   Job ID: $JOB_ID"
echo "   Waiting for completion..."
arthai job status "$JOB_ID" --wait

# 5. Deploy
echo "🚀 Deploying model..."
DEPLOY_ID=$(arthai deploy --model "$MODEL_ID" --endpoint /generate --replicas 2 | grep -oP 'deploy-\S+')
ENDPOINT=$(arthai deployment status "$DEPLOY_ID" | grep -oP 'https://\S+')

# 6. Infer
echo "🎯 Running inference..."
arthai infer --model "$MODEL_ID" --input "Hello, ArthaChain!" --out response.json

echo "✅ Complete! Endpoint: $ENDPOINT"
```

## SDK Usage (Python)

```python
from arthapy import ArthaID, ArthaDataset, ArthaModel, ArthaJob, ArthaDeployment

# Initialize
id_service = ArthaID("http://localhost:8080", "http://localhost:8545")
dataset = ArthaDataset("http://localhost:8080")
model = ArthaModel("http://localhost:8080")
job = ArthaJob("http://localhost:8080")
deploy = ArthaDeployment("http://localhost:8080")

# 1. Create DID
result = id_service.create_did("auth_key", "enc_key", "meta_cid")
did = result["did"]

# 2. Register dataset
dataset_id = dataset.register("artha://bafyData", "artha://bafyLicense", ["tag1", "tag2"])

# 3. Register model
model_id = model.register(
    model_cid="artha://bafyModel",
    architecture="llama",
    dataset_id=dataset_id,
    code_hash="0x123...",
    version="v1"
)

# 4. Train
result = job.submit_train(
    model_id=model_id,
    dataset_id=dataset_id,
    submitter_did=did,
    epochs=3,
    batch_size=64,
    learning_rate=0.001,
    optimizer="adamw",
    budget=600
)
job_id = result["job_id"]

# 5. Deploy
result = deploy.deploy(model_id, "/generate", replicas=2, max_tokens=2048)
endpoint = result["endpoint_url"]

# 6. Infer (via HTTP)
import requests
response = requests.post(endpoint, json={"prompt": "Hello!"})
print(response.json())
```

## SDK Usage (TypeScript)

```typescript
import { ArthaID, ArthaDataset, ArthaModel, ArthaJob, ArthaDeployment } from 'arthajs';

// Initialize
const idService = new ArthaID("http://localhost:8080", "http://localhost:8545");
const dataset = new ArthaDataset("http://localhost:8080");
const model = new ArthaModel("http://localhost:8080");
const job = new ArthaJob("http://localhost:8080");
const deploy = new ArthaDeployment("http://localhost:8080");

// 1. Create DID
const { did } = await idService.createDID("auth_key", "enc_key", "meta_cid");

// 2. Register dataset
const datasetId = await dataset.register("artha://bafyData", "artha://bafyLicense", ["tag1"]);

// 3. Register model
const modelId = await model.register({
    modelCid: "artha://bafyModel",
    architecture: "llama",
    datasetId: datasetId,
    codeHash: "0x123...",
    version: "v1"
});

// 4. Train
const { jobId } = await job.submitTrain({
    modelId,
    datasetId,
    submitterDid: did,
    epochs: 3,
    batchSize: 64,
    learningRate: 0.001,
    optimizer: "adamw",
    budget: 600
});

// 5. Deploy
const { endpointUrl } = await deploy.deploy({
    modelId,
    endpoint: "/generate",
    replicas: 2,
    maxTokens: 2048
});

// 6. Infer
const response = await fetch(endpointUrl, {
    method: "POST",
    body: JSON.stringify({ prompt: "Hello!" })
});
const result = await response.json();
console.log(result);
```

## Common Workflows

### Agentic AI

```bash
# Run autonomous agent
arthai agent run \
    --aiid aiid:agent123 \
    --goal "Automate growth report generation" \
    --tools search,storage,tx \
    --memory episodic
```

### Federated Learning

```bash
# Start federated learning round
arthai fed start \
    --model mid:789 \
    --datasets ds:1,ds:2,ds:3 \
    --rounds 10 \
    --dp on
```

### Evolutionary Search

```bash
# Evolutionary architecture search
arthai evo start \
    --space ./search-space.json \
    --pop 50 \
    --gens 30
```

## Next Steps

- 📖 [Full Documentation](./ARTHA_AIN_V1_SPEC.md)
- 🔧 [API Reference](./API_REFERENCE.md)
- 🎯 [Domain Packs](../domain_packs/)
- 🧪 [Test Suite](../tests/)

## Troubleshooting

**Issue:** "Connection refused"
- **Fix:** Ensure node is running: `cargo run --bin artha_node`

**Issue:** "Job stuck in queued"
- **Fix:** Check GPU availability: `nvidia-smi`

**Issue:** "Policy check failed"
- **Fix:** Ensure DID has required VCs: `arthai vc verify <vc_hash>`

## Support

- 📧 Discord: [ArthaChain Community](https://discord.gg/arthachain)
- 📚 Docs: https://docs.arthachain.online
- 🐛 Issues: https://github.com/arthachain/issues

