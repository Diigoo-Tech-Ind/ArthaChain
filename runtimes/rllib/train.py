#!/usr/bin/env python3
"""Reinforcement Learning Training Script"""
import os
import gymnasium as gym
from stable_baselines3 import PPO
from stable_baselines3.common.callbacks import CheckpointCallback
import requests

JOB_ID = os.environ.get("ARTHA_JOB_ID", "unknown")
STEPS = int(os.environ.get("STEPS", "10000"))
MODEL_PATH = "/model"
PROOF_SERVICE_URL = os.environ.get("PROOF_SERVICE_URL", "http://localhost:8085")

def submit_train_proof(step: int, reward: float, digest: str):
    try:
        requests.post(f"{PROOF_SERVICE_URL}/proof/submit",
            json={"job_id": JOB_ID, "proof_type": "TrainStep", "step": step, "loss": -reward}, timeout=10)
    except:
        pass

def main():
    print(f"🚀 RL Training - Job: {JOB_ID}")
    
    # Create environment
    env = gym.make("CartPole-v1")
    
    # Create model
    model = PPO("MlpPolicy", env, verbose=1)
    
    # Training callback
    checkpoint_callback = CheckpointCallback(
        save_freq=1000, save_path=MODEL_PATH, name_prefix="rl_model"
    )
    
    # Train
    model.learn(total_timesteps=STEPS, callback=checkpoint_callback, progress_bar=True)
    
    # Save final model
    model.save(os.path.join(MODEL_PATH, "final_model"))
    
    print(f"✅ Training complete!")

if __name__ == "__main__":
    main()

