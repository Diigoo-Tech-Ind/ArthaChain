#!/usr/bin/env python3
"""
ArthaAIN v1 - Checkpoint Saver Utility
Uploads checkpoints to SVDB and records metadata
"""

import os
import sys
import json
import time
import requests

JOB_ID = os.environ.get("ARTHA_JOB_ID", "unknown")
CHECKPOINT_DIR = "/checkpoints"
SVDB_API_URL = os.environ.get("SVDB_API_URL", "http://localhost:8080")

def save_checkpoint(checkpoint_path: str, metrics: dict, step: int):
    """Save checkpoint to SVDB and return CID"""
    print(f"💾 Saving checkpoint at step {step}...")
    
    # Upload to SVDB
    try:
        with open(checkpoint_path, 'rb') as f:
            files = {'file': (os.path.basename(checkpoint_path), f)}
            data = {
                'replicas': '5',
                'months': '12',
            }
            response = requests.post(
                f"{SVDB_API_URL}/svdb/upload",
                files=files,
                data=data,
                timeout=300
            )
            
            if response.status_code == 200:
                result = response.json()
                checkpoint_cid = result.get('cid', '')
                print(f"   ✅ Uploaded to SVDB: {checkpoint_cid}")
                
                # Save metadata
                metadata = {
                    "job_id": JOB_ID,
                    "step": step,
                    "checkpoint_cid": checkpoint_cid,
                    "metrics": metrics,
                    "timestamp": time.time(),
                }
                
                metadata_path = checkpoint_path.replace('.pt', '.json')
                with open(metadata_path, 'w') as mf:
                    json.dump(metadata, mf, indent=2)
                
                return checkpoint_cid
            else:
                print(f"   ⚠️  Upload failed: {response.status_code}")
                return None
    except Exception as e:
        print(f"   ⚠️  Upload error: {e}")
        return None

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: checkpoint_saver.py <checkpoint_path> <step> [metrics_json]")
        sys.exit(1)
    
    checkpoint_path = sys.argv[1]
    step = int(sys.argv[2])
    metrics = json.loads(sys.argv[3]) if len(sys.argv) > 3 else {}
    
    cid = save_checkpoint(checkpoint_path, metrics, step)
    if cid:
        print(f"Checkpoint CID: {cid}")
        sys.exit(0)
    else:
        sys.exit(1)

