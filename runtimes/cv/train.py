#!/usr/bin/env python3
"""Computer Vision Training Script"""
import os
import torch
import torch.nn as nn
from torchvision import transforms
from ultralytics import YOLO
import requests

JOB_ID = os.environ.get("ARTHA_JOB_ID", "unknown")
EPOCHS = int(os.environ.get("EPOCHS", "3"))
MODEL_PATH = "/model"
DATA_PATH = "/data"
PROOF_SERVICE_URL = os.environ.get("PROOF_SERVICE_URL", "http://localhost:8085")

def submit_train_proof(step: int, loss: float, digest: str):
    try:
        requests.post(f"{PROOF_SERVICE_URL}/proof/submit",
            json={"job_id": JOB_ID, "proof_type": "TrainStep", "step": step, "loss": loss}, timeout=10)
    except:
        pass

def main():
    print(f"🚀 CV Training - Job: {JOB_ID}")
    
    # Load or create model
    try:
        model = YOLO(os.path.join(MODEL_PATH, "model.pt"))
    except:
        model = YOLO("yolo11n.pt")  # Default
    
    # Train
    if os.path.exists(DATA_PATH):
        results = model.train(
            data=os.path.join(DATA_PATH, "dataset.yaml"),
            epochs=EPOCHS,
            imgsz=640,
            device=0 if torch.cuda.is_available() else "cpu"
        )
        
        for i, result in enumerate(results):
            if hasattr(result, "loss"):
                submit_train_proof(i, float(result.loss), f"0x{hash(str(i)):016x}")
        
        model.save(os.path.join(MODEL_PATH, "trained.pt"))
    else:
        print("⚠️  No dataset found, skipping training")
    
    print(f"✅ Training complete!")

if __name__ == "__main__":
    main()

