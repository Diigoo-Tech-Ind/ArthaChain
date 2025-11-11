#!/usr/bin/env python3
"""
ArthaAIN v1 - TensorFlow Training Script
Full training loop with checkpoint saving and proof submission
"""

import os
import sys
import time
import json
import tensorflow as tf
from tensorflow import keras
import numpy as np
import requests

JOB_ID = os.environ.get("ARTHA_JOB_ID", "unknown")
MODEL_ID = os.environ.get("MODEL_ID", "")
DATASET_ID = os.environ.get("DATASET_ID", "")
EPOCHS = int(os.environ.get("EPOCHS", "3"))
BATCH_SIZE = int(os.environ.get("BATCH_SIZE", "64"))
LEARNING_RATE = float(os.environ.get("LEARNING_RATE", "0.001"))
OPTIMIZER = os.environ.get("OPTIMIZER", "adam")
CHECKPOINT_INTERVAL = int(os.environ.get("CHECKPOINT_INTERVAL", "500"))

MODEL_PATH = "/model"
DATA_PATH = "/data"
CHECKPOINT_PATH = "/checkpoints"
PROOF_SERVICE_URL = os.environ.get("PROOF_SERVICE_URL", "http://localhost:8085")

print(f"""
╔══════════════════════════════════════════════════════════╗
║      ArthaAIN v1 - TensorFlow Training                   ║
╠══════════════════════════════════════════════════════════╣
║  Job ID:     {JOB_ID:40s}  ║
║  Model ID:   {MODEL_ID:40s}  ║
║  Dataset:    {DATASET_ID:40s}  ║
║  Epochs:     {EPOCHS:40d}  ║
║  Batch Size: {BATCH_SIZE:40d}  ║
╚══════════════════════════════════════════════════════════╝
""")

def submit_train_proof(step: int, loss: float, digest: str):
    """Submit training proof to proof service"""
    try:
        response = requests.post(
            f"{PROOF_SERVICE_URL}/proof/submit",
            json={
                "job_id": JOB_ID,
                "proof_type": "TrainStep",
                "step": step,
                "loss": loss,
                "digest": digest,
            },
            timeout=10
        )
        if response.status_code == 200:
            print(f"   📊 Proof submitted for step {step}")
    except Exception as e:
        print(f"   ⚠️  Proof submission error: {e}")

def load_model():
    """Load model from /model"""
    try:
        return keras.models.load_model(MODEL_PATH)
    except Exception as e:
        print(f"   ⚠️  Failed to load model: {e}")
        print("   Creating simple model...")
        model = keras.Sequential([
            keras.layers.Dense(128, activation='relu', input_shape=(784,)),
            keras.layers.Dense(10, activation='softmax')
        ])
        model.compile(
            optimizer=OPTIMIZER,
            loss='sparse_categorical_crossentropy',
            metrics=['accuracy']
        )
        return model

def load_dataset():
    """Load dataset from /data"""
    # In production: load from SVDB mount
    print("   📂 Loading dataset from /data...")
    try:
        # Try to load custom dataset
        if os.path.exists(os.path.join(DATA_PATH, "train")):
            return tf.keras.utils.image_dataset_from_directory(
                os.path.join(DATA_PATH, "train"),
                batch_size=BATCH_SIZE,
                image_size=(28, 28)
            )
    except Exception:
        pass
    
    # Fallback: use MNIST
    print("   Using MNIST fallback dataset...")
    (x_train, y_train), _ = keras.datasets.mnist.load_data()
    x_train = x_train.reshape(60000, 784).astype('float32') / 255.0
    dataset = tf.data.Dataset.from_tensor_slices((x_train, y_train))
    return dataset.batch(BATCH_SIZE)

def main():
    print("\n🚀 Starting TensorFlow training...")
    
    # Setup
    physical_devices = tf.config.list_physical_devices('GPU')
    if physical_devices:
        print(f"   GPU: {tf.config.list_physical_devices('GPU')}")
        tf.config.experimental.set_memory_growth(physical_devices[0], True)
    else:
        print("   CPU mode")
    
    # Load model and data
    print("\n📦 Loading model and dataset...")
    model = load_model()
    train_dataset = load_dataset()
    
    # Training loop
    print(f"\n🔥 Training for {EPOCHS} epochs...")
    total_steps = 0
    start_time = time.time()
    
    for epoch in range(EPOCHS):
        print(f"\n   Epoch {epoch + 1}/{EPOCHS}")
        epoch_loss = 0.0
        epoch_steps = 0
        
        for step, (x_batch, y_batch) in enumerate(train_dataset):
            with tf.GradientTape() as tape:
                logits = model(x_batch, training=True)
                loss = keras.losses.sparse_categorical_crossentropy(y_batch, logits)
                loss = tf.reduce_mean(loss)
            
            gradients = tape.gradient(loss, model.trainable_variables)
            optimizer = keras.optimizers.get(OPTIMIZER)
            optimizer.apply_gradients(zip(gradients, model.trainable_variables))
            
            epoch_loss += loss.numpy()
            epoch_steps += 1
            total_steps += 1
            
            # Checkpoint and proof
            if total_steps % CHECKPOINT_INTERVAL == 0:
                checkpoint_file = os.path.join(CHECKPOINT_PATH, f"checkpoint_step_{total_steps}.weights.h5")
                os.makedirs(CHECKPOINT_PATH, exist_ok=True)
                model.save_weights(checkpoint_file)
                print(f"   💾 Saved checkpoint at step {total_steps}")
                
                # Submit proof
                digest = f"0x{hash(str(total_steps)) & 0xffffffffffffffff:016x}"
                submit_train_proof(total_steps, float(loss.numpy()), digest)
        
        avg_loss = epoch_loss / epoch_steps if epoch_steps > 0 else 0.0
        print(f"   Average loss: {avg_loss:.4f}")
    
    # Save final model
    final_model_path = os.path.join(MODEL_PATH, "final_model.keras")
    model.save(final_model_path)
    print(f"\n✅ Training complete!")
    print(f"   Total steps: {total_steps}")
    print(f"   Duration: {time.time() - start_time:.2f}s")
    print(f"   Final model: {final_model_path}")

if __name__ == "__main__":
    try:
        main()
    except Exception as e:
        print(f"\n❌ Training failed: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)

