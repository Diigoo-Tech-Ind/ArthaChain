# ArthaAIN v1 — Complete API Reference

## Base URL

All endpoints are available at: `http://localhost:8080` (default)

## Authentication

Most endpoints require authentication via:
- `X-Artha-DID`: DID identifier
- `X-Artha-Session`: Session token (from `/policy/session/create`)

## REST Endpoints

### Storage (SVDB)

#### `POST /svdb/upload`
Upload file/directory to SVDB.

**Request:**
```json
{
  "file": "<multipart/form-data>",
  "replicas": 5,
  "months": 12,
  "envelope": "<optional encryption envelope>"
}
```

**Response:**
```json
{
  "cid": "artha://bafyBeibaz123...",
  "size": 1048576,
  "merkleRoot": "0x...",
  "poseidonRoot": "0x..."
}
```

#### `GET /svdb/download/:cid`
Download file by CID.

**Headers:**
- `Range`: Optional byte range (`bytes=0-1023`)

**Response:**
Binary file data

#### `POST /svdb/deals`
Create storage deal.

**Request:**
```json
{
  "cid": "artha://...",
  "size": 1048576,
  "replicas": 5,
  "months": 12,
  "maxPricePerGBMonth": "1000000000000000000"
}
```

### Identity

#### `POST /identity/did/create`
Create new DID.

**Request:**
```json
{
  "authKey": "0x...",
  "encKey": "0x...",
  "metaCid": "artha://..."
}
```

**Response:**
```json
{
  "did": "did:artha:abc123...",
  "txHash": "0x..."
}
```

#### `GET /identity/did/:did`
Get DID document.

#### `POST /identity/did/rotate`
Rotate DID keys.

#### `POST /identity/vc/issue`
Issue verifiable credential.

**Request:**
```json
{
  "issuerDid": "did:artha:...",
  "subjectDid": "did:artha:...",
  "claimHash": "0x...",
  "docCid": "artha://...",
  "expiresAt": 1234567890
}
```

### AI Jobs

#### `POST /ai/dataset/register`
Register dataset.

**Request:**
```json
{
  "rootCid": "artha://...",
  "licenseCid": "artha://...",
  "tags": ["medical", "imaging"]
}
```

**Response:**
```json
{
  "datasetId": "ds:artha:xyz456"
}
```

#### `POST /ai/model/register`
Register AI model.

**Request:**
```json
{
  "modelCid": "artha://...",
  "architecture": "llama",
  "baseModelId": null,
  "datasetId": "ds:artha:xyz",
  "codeHash": "0x123...",
  "version": "v1"
}
```

**Response:**
```json
{
  "modelId": "mid:789"
}
```

#### `POST /ai/train`
Submit training job.

**Request:**
```json
{
  "modelId": "mid:789",
  "datasetId": "ds:artha:xyz",
  "submitterDid": "did:artha:...",
  "params": {
    "epochs": 3,
    "batchSize": 64,
    "learningRate": 0.001,
    "optimizer": "adamw",
    "checkpointInterval": 500
  },
  "budget": 600
}
```

**Response:**
```json
{
  "jobId": "job-abc123",
  "status": "queued",
  "estimatedCost": 550,
  "estimatedDurationSecs": 3600
}
```

#### `POST /ai/infer`
Submit inference job.

**Request:**
```json
{
  "modelId": "mid:789",
  "inputCid": "artha://...",
  "inlineInput": "Hello!",
  "submitterDid": "did:artha:...",
  "mode": "chat",
  "maxTokens": 2048,
  "budget": 10
}
```

#### `POST /ai/agent`
Submit agent job.

**Request:**
```json
{
  "agentSpecCid": "artha://...",
  "submitterDid": "did:artha:...",
  "goal": "Complete the task",
  "tools": ["search", "storage", "tx"],
  "memoryPolicy": "episodic",
  "budget": 100
}
```

#### `GET /ai/job/:jobId/status`
Get job status.

**Response:**
```json
{
  "job": {
    "jobId": "job-abc123",
    "status": "Running",
    "progress": 0.65,
    "outputCid": "artha://...",
    "artifacts": ["artha://checkpoint1", "artha://checkpoint2"]
  }
}
```

#### `GET /ai/job/:jobId/logs`
Get job logs.

**Response:**
```json
[
  "Starting training...",
  "Epoch 1/3: Loss 0.5234",
  "..."
]
```

### Federated Learning

#### `POST /ai/federated/start`
Start federated learning round.

**Request:**
```json
{
  "modelId": "mid:789",
  "datasetIds": ["ds:1", "ds:2", "ds:3"],
  "rounds": 10,
  "dp": true,
  "budget": 500
}
```

**Response:**
```json
{
  "fedId": "fed-xyz789",
  "status": "queued"
}
```

#### `GET /ai/federated/:fedId/status`
Get federated learning status.

#### `POST /ai/federated/:fedId/gradient`
Submit gradient update.

#### `POST /ai/federated/:fedId/aggregate`
Trigger aggregation.

### Evolutionary AI

#### `POST /ai/evolve/start`
Start evolutionary search.

**Request:**
```json
{
  "searchSpaceCid": "artha://...",
  "population": 50,
  "generations": 30,
  "budget": 1000
}
```

#### `GET /ai/evolve/:evoId/status`
Get evolution status.

#### `GET /ai/evolve/:evoId/population`
Get population status.

### Model Deployment

#### `POST /ai/deploy`
Deploy model for inference.

**Request:**
```json
{
  "modelId": "mid:789",
  "endpoint": "/generate",
  "replicas": 2,
  "maxTokens": 2048
}
```

**Response:**
```json
{
  "deploymentId": "deploy-xyz",
  "endpointUrl": "https://ain.artha.online/mid:789/v1"
}
```

#### `GET /ai/deployment/:deploymentId/status`
Get deployment status.

#### `POST /ai/deployment/:deploymentId/scale`
Scale deployment.

**Request:**
```json
{
  "replicas": 5
}
```

#### `DELETE /ai/deployment/:deploymentId`
Undeploy model.

### Agents

#### `GET /agents/:jobId/tool-calls`
Get agent tool calls.

#### `POST /agents/:jobId/tool-call`
Record tool call.

#### `GET /agents/:jobId/memory`
Get agent memory.

#### `POST /agents/:jobId/memory`
Update agent memory.

### Policy

#### `POST /policy/check`
Check access policy.

**Request:**
```json
{
  "did": "did:artha:...",
  "action": "train",
  "resourceCid": "artha://...",
  "datasetId": "ds:artha:...",
  "modelId": "mid:..."
}
```

**Response:**
```json
{
  "allowed": true,
  "reasons": ["PASS: Valid DID", "PASS: Required VCs present"]
}
```

#### `POST /policy/session/create`
Create session token.

#### `POST /policy/session/revoke`
Revoke session token.

### Dashboard

#### `GET /api/dashboard/stats`
Get aggregated system statistics.

**Response:**
```json
{
  "jobs": {
    "total": 1234,
    "running": 23,
    "completed": 1100,
    "failed": 50,
    "queued": 61
  },
  "models": {
    "total": 42,
    "deployed": 12,
    "training": 3,
    "published": 27
  },
  "compute": {
    "active_gpus": 24,
    "total_gpu_hours": 15234.5,
    "avg_latency_ms": 145.2,
    "jobs_per_hour": 12.5
  },
  "storage": {
    "total_gb": 5000.0,
    "used_gb": 3750.0,
    "replicas": 5,
    "active_providers": 48
  },
  "policy": {
    "checks_today": 125000,
    "allowed": 120000,
    "denied": 5000,
    "avg_latency_ms": 98.5
  }
}
```

## Error Responses

All errors follow this format:

```json
{
  "error": {
    "code": 400,
    "message": "Invalid request",
    "details": {
      "field": "modelId",
      "reason": "Model not found"
    }
  }
}
```

## Rate Limits

- **Default**: 100 requests/minute per DID
- **Storage**: 10 uploads/minute
- **Training**: 5 jobs/minute
- **Inference**: 100 requests/minute

## SDK Usage

### TypeScript
```typescript
import { ArthaJS, ArthaID, ArthaModel, ArthaJob } from 'arthajs';

const client = new ArthaJS('http://localhost:8080');
const model = new ArthaModel('http://localhost:8080');

const modelId = await model.register({...});
const job = await model.submitTrain({...});
```

### Python
```python
from arthapy import ArthaModel, ArthaJob

model = ArthaModel('http://localhost:8080')
model_id = model.register(...)
job = model.submit_train(...)
```

## WebSocket Streaming

### `ws://localhost:8080/ws/job/:jobId/logs`
Stream job logs in real-time.

### `ws://localhost:8080/ws/job/:jobId/status`
Stream job status updates.

## Examples

See [DEV_QUICKSTART.md](./DEV_QUICKSTART.md) for complete examples.

