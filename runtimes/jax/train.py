#!/usr/bin/env python3
"""JAX Training Script"""
import os
import sys
import time
import jax
import jax.numpy as jnp
from flax import linen as nn
import optax
import requests

JOB_ID = os.environ.get("ARTHA_JOB_ID", "unknown")
EPOCHS = int(os.environ.get("EPOCHS", "3"))
BATCH_SIZE = int(os.environ.get("BATCH_SIZE", "64"))
LEARNING_RATE = float(os.environ.get("LEARNING_RATE", "0.001"))
MODEL_PATH = "/model"
DATA_PATH = "/data"
CHECKPOINT_PATH = "/checkpoints"
PROOF_SERVICE_URL = os.environ.get("PROOF_SERVICE_URL", "http://localhost:8085")

class SimpleModel(nn.Module):
    @nn.compact
    def __call__(self, x):
        x = nn.Dense(128)(x)
        x = nn.relu(x)
        x = nn.Dense(10)(x)
        return x

def submit_train_proof(step: int, loss: float, digest: str):
    try:
        requests.post(
            f"{PROOF_SERVICE_URL}/proof/submit",
            json={"job_id": JOB_ID, "proof_type": "TrainStep", "step": step, "loss": loss},
            timeout=10
        )
    except:
        pass

def main():
    print(f"🚀 JAX Training - Job: {JOB_ID}")
    
    # Initialize model
    model = SimpleModel()
    key = jax.random.PRNGKey(0)
    dummy_input = jnp.ones((1, 784))
    params = model.init(key, dummy_input)
    
    # Optimizer
    optimizer = optax.adam(LEARNING_RATE)
    opt_state = optimizer.init(params)
    
    # Training loop
    for epoch in range(EPOCHS):
        for step in range(100):  # Simplified
            # Dummy training step
            key, subkey = jax.random.split(key)
            dummy_x = jax.random.normal(subkey, (BATCH_SIZE, 784))
            dummy_y = jnp.zeros((BATCH_SIZE, 10))
            
            def loss_fn(p):
                logits = model.apply(p, dummy_x)
                return jnp.mean((logits - dummy_y) ** 2)
            
            loss, grads = jax.value_and_grad(loss_fn)(params)
            updates, opt_state = optimizer.update(grads, opt_state)
            params = optax.apply_updates(params, updates)
            
            if step % 50 == 0:
                submit_train_proof(step, float(loss), f"0x{hash(str(step)):016x}")
                print(f"   Step {step}: loss = {loss:.4f}")
        
        print(f"   Epoch {epoch + 1}/{EPOCHS} complete")
    
    print(f"✅ Training complete!")

if __name__ == "__main__":
    main()

