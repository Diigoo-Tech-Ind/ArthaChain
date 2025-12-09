# ArthaChain Python SDK

Official Python SDK for ArthaChain - Identity, Storage, and AI Cloud Platform.

## Installation

```bash
pip install arthapy
```

## Quick Start

```python
from arthapy import ArthaPy, ArthaID, ArthaVC, ArthaJob

# Connect to ArthaChain node
artha = ArthaPy("http://localhost:8080")

# Upload a file to SVDB (decentralized storage)
cid = artha.upload_file("my_data.json")
print(f"Stored at: artha://{cid}")

# Download a file
artha.download(f"artha://{cid}", "downloaded_data.json")

# Get file info
info = artha.info(f"artha://{cid}")
print(f"Size: {info['size']} bytes")
```

## Features

### Storage (SVDB)
```python
# Upload with encryption envelope
cid = artha.upload_file_with_envelope("secret.txt", {
    "alg": "xchacha20-poly1305",
    "nonce_b64": "...",
    "salt_b64": "..."
})

# Range download
artha.download(cid, "partial.bin", start=0, end=1024)

# Access control
artha.set_access_policy(cid=cid, private=True, allowed_dids=["did:artha:0x..."])
```

### Identity (DID/VC)
```python
identity = ArthaID("http://localhost:8080", "http://localhost:8545")

# Create DID
result = identity.create_did(auth_key, enc_key, meta_cid)
print(f"DID: {result['did']}")

# Issue Verifiable Credential
vc = ArthaVC("http://localhost:8080")
vc_hash = vc.issue_vc(issuer_did, subject_did, claim_hash, doc_cid, expires_at)
```

### AI Cloud (ArthaAIN)
```python
job = ArthaJob("http://localhost:8080")

# Submit training job
result = job.submit_train(
    model_id="gpt2-finetuned",
    dataset_id="my-dataset",
    submitter_did="did:artha:0x...",
    epochs=10,
    batch_size=32,
    learning_rate=0.001,
    optimizer="adam",
    budget=1000
)
print(f"Job ID: {result['job_id']}")

# Check status
status = job.get_status(result['job_id'])
print(f"Status: {status['state']}")
```

## API Reference

### ArthaPy (Storage)
- `upload_file(path)` - Upload file to SVDB
- `download(cid, out_path)` - Download file
- `info(cid)` - Get file info
- `set_access_policy(...)` - Set access controls
- `ai_train(...)` - Submit AI training job
- `ai_deploy(...)` - Deploy AI model

### ArthaID (Identity)
- `create_did(...)` - Create decentralized identity
- `get_did(did)` - Get DID document
- `rotate_keys(...)` - Rotate authentication keys
- `verify_signature(...)` - Verify signature

### ArthaVC (Credentials)
- `issue_vc(...)` - Issue verifiable credential
- `revoke_vc(...)` - Revoke credential
- `verify_vc(...)` - Verify credential
- `has_claim_type(...)` - Check claim existence

### ArthaJob (AI Jobs)
- `submit_train(...)` - Submit training job
- `submit_infer(...)` - Submit inference job
- `submit_agent(...)` - Submit AI agent task
- `get_status(...)` - Get job status
- `get_logs(...)` - Get job logs

## License

MIT License - see LICENSE file.
