#!/usr/bin/env python3
"""Recommendation System Training Script"""
import os
import numpy as np
from lightfm import LightFM
from lightfm.datasets import fetch_movielens
from lightfm.evaluation import auc_score
import requests

JOB_ID = os.environ.get("ARTHA_JOB_ID", "unknown")
EPOCHS = int(os.environ.get("EPOCHS", "10"))
MODEL_PATH = "/model"
PROOF_SERVICE_URL = os.environ.get("PROOF_SERVICE_URL", "http://localhost:8085")

def submit_train_proof(epoch: int, loss: float, digest: str):
    try:
        requests.post(f"{PROOF_SERVICE_URL}/proof/submit",
            json={"job_id": JOB_ID, "proof_type": "TrainStep", "step": epoch, "loss": loss}, timeout=10)
    except:
        pass

def main():
    print(f"🚀 Recommendation Training - Job: {JOB_ID}")
    
    # Load data
    data = fetch_movielens(min_rating=4.0)
    train = data['train']
    test = data['test']
    
    # Create model
    model = LightFM(no_components=30, learning_rate=0.05, loss='warp')
    
    # Train
    for epoch in range(EPOCHS):
        model.fit_partial(train, epochs=1)
        train_auc = auc_score(model, train).mean()
        test_auc = auc_score(model, test).mean()
        
        loss = 1.0 - train_auc
        submit_train_proof(epoch, loss, f"0x{hash(str(epoch)):016x}")
        print(f"   Epoch {epoch + 1}: train_auc={train_auc:.4f}, test_auc={test_auc:.4f}")
    
    # Save
    os.makedirs(MODEL_PATH, exist_ok=True)
    import pickle
    with open(os.path.join(MODEL_PATH, "model.pkl"), 'wb') as f:
        pickle.dump(model, f)
    
    print(f"✅ Training complete!")

if __name__ == "__main__":
    main()

