#!/usr/bin/env python3
"""RLlib Training - Real Ray RLlib Implementation"""

import os
import ray
from ray.rllib.algorithms.ppo import PPOConfig
from ray.rllib.algorithms.dqn import DQNConfig
from ray.rllib.env import gymnasium
import requests

JOB_ID = os.environ.get("ARTHA_JOB_ID", "rllib-unknown")
ALGORITHM = os.environ.get("RL_ALGORITHM", "PPO")
ENV = os.environ.get("RL_ENV", "CartPole-v1")
STEPS = int(os.environ.get("STEPS", "10000"))
PROOF_SERVICE_URL = os.environ.get("PROOF_SERVICE_URL", "http://localhost:8085")

def submit_proof(step: int, reward: float):
    try:
        requests.post(
            f"{PROOF_SERVICE_URL}/proof/submit",
            json={"job_id": JOB_ID, "proof_type": "RLStep", "step": step, "reward": reward},
            timeout=10,
        )
    except:
        pass

def main():
    print(f"🚀 RLlib Training - Job: {JOB_ID}")
    
    # Initialize Ray
    if not ray.is_initialized():
        ray.init(ignore_reinit_error=True)
    
    # Create algorithm config
    if ALGORITHM == "PPO":
        algo = (
            PPOConfig()
            .environment(ENV)
            .framework("torch")
            .rollouts(num_rollout_workers=2)
            .build()
        )
    elif ALGORITHM == "DQN":
        algo = (
            DQNConfig()
            .environment(ENV)
            .framework("torch")
            .build()
        )
    else:
        raise ValueError(f"Unknown algorithm: {ALGORITHM}")
    
    # Train
    print(f"   Training {ALGORITHM} on {ENV} for {STEPS} steps...")
    
    for step in range(STEPS):
        result = algo.train()
        
        if step % 100 == 0:
            reward = result.get("episode_reward_mean", 0)
            submit_proof(step, reward)
            print(f"   Step {step}: Reward = {reward:.2f}")
    
    # Save model
    checkpoint_path = algo.save("/tmp/rllib_checkpoint")
    print(f"✅ Training complete! Checkpoint: {checkpoint_path}")
    
    # Upload to SVDB
    sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    try:
        from svdb_client import svdb
        cid = svdb.upload(checkpoint_path, replicas=3, months=6)
        print(f"   Uploaded to SVDB: {cid}")
    except:
        pass
    
    ray.shutdown()

if __name__ == "__main__":
    import sys
    main()

