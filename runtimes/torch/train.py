#!/usr/bin/env python3
"""
ArthaAIN v1 - PyTorch Training Script
Reads model/data from SVDB mounts, trains, saves checkpoints
"""

import os
import sys
import time
import json
import torch
import torch.nn as nn
from torch.utils.data import DataLoader, Dataset
from transformers import AutoModel, AutoTokenizer, get_linear_schedule_with_warmup
import requests

# Environment variables from container
JOB_ID = os.environ.get("ARTHA_JOB_ID", "unknown")
EPOCHS = int(os.environ.get("EPOCHS", "1"))
BATCH_SIZE = int(os.environ.get("BATCH_SIZE", "32"))
LEARNING_RATE = float(os.environ.get("LEARNING_RATE", "0.001"))
OPTIMIZER_NAME = os.environ.get("OPTIMIZER", "adam")
CHECKPOINT_INTERVAL = int(os.environ.get("CHECKPOINT_INTERVAL", "500"))

# Paths (mounted by ai-runtime)
MODEL_PATH = "/model"
DATA_PATH = "/data"
CHECKPOINT_PATH = "/checkpoints"

# Proof service URL
PROOF_SERVICE_URL = os.environ.get("PROOF_SERVICE_URL", "http://localhost:8085")

print(f"""
╔══════════════════════════════════════════════════════════╗
║          ArthaAIN v1 - PyTorch Training                  ║
╠══════════════════════════════════════════════════════════╣
║  Job ID:     {JOB_ID:40s}  ║
║  Epochs:     {EPOCHS:40d}  ║
║  Batch Size: {BATCH_SIZE:40d}  ║
║  LR:         {LEARNING_RATE:40.6f}  ║
║  Optimizer:  {OPTIMIZER_NAME:40s}  ║
╚══════════════════════════════════════════════════════════╝
""")

# Simple dataset (in production: load from /data)
class SimpleDataset(Dataset):
    def __init__(self, size=1000):
        self.size = size
        self.data = torch.randn(size, 128)
        self.labels = torch.randint(0, 10, (size,))
    
    def __len__(self):
        return self.size
    
    def __getitem__(self, idx):
        return self.data[idx], self.labels[idx]

# Simple model (in production: load from /model)
class SimpleModel(nn.Module):
    def __init__(self):
        super().__init__()
        self.fc1 = nn.Linear(128, 256)
        self.fc2 = nn.Linear(256, 128)
        self.fc3 = nn.Linear(128, 10)
        self.relu = nn.ReLU()
    
    def forward(self, x):
        x = self.relu(self.fc1(x))
        x = self.relu(self.fc2(x))
        return self.fc3(x)

def submit_proof(step, loss, gradients, weights):
    """Submit training proof to proof service"""
    try:
        response = requests.post(
            f"{PROOF_SERVICE_URL}/proof/submit",
            json={
                "job_id": JOB_ID,
                "proof_type": "TrainStep",
                "step": step,
                "loss": float(loss),
                "gradients": gradients[:10],  # First 10 gradient values
                "weights": weights[:10],      # First 10 weight values
            },
            timeout=10
        )
        if response.status_code == 200:
            print(f"   📊 Proof submitted for step {step}")
        else:
            print(f"   ⚠️  Proof submission failed: {response.status_code}")
    except Exception as e:
        print(f"   ⚠️  Proof submission error: {e}")

def save_checkpoint(model, optimizer, epoch, step, loss):
    """Save checkpoint and auto-push to SVDB"""
    checkpoint_file = os.path.join(CHECKPOINT_PATH, f"checkpoint-epoch{epoch}-step{step}.pt")
    
    torch.save({
        'epoch': epoch,
        'step': step,
        'model_state_dict': model.state_dict(),
        'optimizer_state_dict': optimizer.state_dict(),
        'loss': loss,
    }, checkpoint_file)
    
    print(f"   💾 Checkpoint saved: epoch{epoch}-step{step}")
    
    # Save metadata
    metadata = {
        "job_id": JOB_ID,
        "epoch": epoch,
        "step": step,
        "loss": float(loss),
        "timestamp": time.time(),
    }
    
    metadata_file = os.path.join(CHECKPOINT_PATH, f"checkpoint-epoch{epoch}-step{step}.json")
    with open(metadata_file, 'w') as f:
        json.dump(metadata, f, indent=2)
    
    # Auto-push checkpoint to SVDB
    try:
        sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
        from svdb_client import svdb
        
        checkpoint_cid = svdb.upload(checkpoint_file, replicas=3, months=6)
        if checkpoint_cid:
            print(f"   ✅ Checkpoint uploaded to SVDB: {checkpoint_cid}")
            
            # Also upload metadata
            metadata_cid = svdb.upload(metadata_file, replicas=3, months=6)
            if metadata_cid:
                print(f"   ✅ Metadata uploaded: {metadata_cid}")
    except Exception as e:
        print(f"   ⚠️  SVDB upload failed: {e}")

def main():
    print("\n🚀 Starting training...")
    
    # Setup device
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"   Device: {device}")
    
    if torch.cuda.is_available():
        print(f"   GPU: {torch.cuda.get_device_name(0)}")
        print(f"   VRAM: {torch.cuda.get_device_properties(0).total_memory / 1e9:.2f} GB")
    
    # Load dataset
    print("\n📂 Loading dataset...")
    dataset = SimpleDataset(size=10000)
    dataloader = DataLoader(dataset, batch_size=BATCH_SIZE, shuffle=True)
    print(f"   Samples: {len(dataset)}")
    print(f"   Batches: {len(dataloader)}")
    
    # Load model
    print("\n🧠 Loading model...")
    model = SimpleModel().to(device)
    print(f"   Parameters: {sum(p.numel() for p in model.parameters()):,}")
    
    # Setup optimizer
    if OPTIMIZER_NAME.lower() == "adam":
        optimizer = torch.optim.Adam(model.parameters(), lr=LEARNING_RATE)
    elif OPTIMIZER_NAME.lower() == "sgd":
        optimizer = torch.optim.SGD(model.parameters(), lr=LEARNING_RATE, momentum=0.9)
    elif OPTIMIZER_NAME.lower() == "adamw":
        optimizer = torch.optim.AdamW(model.parameters(), lr=LEARNING_RATE)
    else:
        optimizer = torch.optim.Adam(model.parameters(), lr=LEARNING_RATE)
    
    criterion = nn.CrossEntropyLoss()
    
    # Training loop
    print(f"\n🔄 Training for {EPOCHS} epochs...")
    global_step = 0
    
    for epoch in range(EPOCHS):
        print(f"\n{'='*60}")
        print(f"Epoch {epoch + 1}/{EPOCHS}")
        print(f"{'='*60}")
        
        model.train()
        epoch_loss = 0.0
        
        for batch_idx, (data, target) in enumerate(dataloader):
            global_step += 1
            
            data, target = data.to(device), target.to(device)
            
            # Forward pass
            optimizer.zero_grad()
            output = model(data)
            loss = criterion(output, target)
            
            # Backward pass
            loss.backward()
            optimizer.step()
            
            epoch_loss += loss.item()
            
            # Log progress
            if batch_idx % 10 == 0:
                print(f"   Step {global_step:5d} | Batch {batch_idx:4d}/{len(dataloader):4d} | Loss: {loss.item():.4f}")
            
            # Auto-submit proof periodically
            if global_step % 100 == 0:
                # Extract gradient and weight samples
                gradients = []
                weights = []
                for param in model.parameters():
                    if param.grad is not None:
                        gradients.extend(param.grad.flatten()[:5].cpu().tolist())
                    weights.extend(param.data.flatten()[:5].cpu().tolist())
                
                submit_proof(global_step, loss.item(), gradients[:10], weights[:10])
            
            # Auto-push checkpoint to SVDB and update lineage
            if global_step % CHECKPOINT_INTERVAL == 0:
                save_checkpoint(model, optimizer, epoch, global_step, loss.item())
                
                # Auto-update model lineage in ModelRegistry
                try:
                    import requests
                    checkpoint_cid = svdb.upload(
                        os.path.join(CHECKPOINT_PATH, f"checkpoint-epoch{epoch}-step{global_step}.pt"),
                        replicas=3,
                        months=6
                    )
                    if checkpoint_cid:
                        # Call ModelRegistry.addCheckpoint()
                        model_registry_url = os.environ.get("MODEL_REGISTRY_URL", "http://localhost:8080")
                        model_id = os.environ.get("MODEL_ID", "")
                        if model_id:
                            requests.post(
                                f"{model_registry_url}/ai/model/{model_id}/checkpoint",
                                json={
                                    "checkpointCid": checkpoint_cid,
                                    "step": global_step,
                                },
                                timeout=5,
                            )
                except Exception as e:
                    print(f"   ⚠️  Lineage update failed: {e}")
        
        # Epoch summary
        avg_loss = epoch_loss / len(dataloader)
        print(f"\n   📊 Epoch {epoch + 1} Summary:")
        print(f"      Average Loss: {avg_loss:.4f}")
        
        # Save epoch checkpoint
        save_checkpoint(model, optimizer, epoch, global_step, avg_loss)
    
    # Final checkpoint
    print(f"\n✅ Training complete!")
    final_checkpoint = os.path.join(CHECKPOINT_PATH, "final-model.pt")
    torch.save({
        'model_state_dict': model.state_dict(),
        'optimizer_state_dict': optimizer.state_dict(),
        'epochs': EPOCHS,
        'final_loss': avg_loss,
    }, final_checkpoint)
    
    print(f"   💾 Final model saved: final-model.pt")
    print(f"   📊 Total steps: {global_step}")
    print(f"   🎯 Final loss: {avg_loss:.4f}")

if __name__ == "__main__":
    try:
        main()
    except Exception as e:
        print(f"\n❌ Training failed: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)

