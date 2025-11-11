#!/usr/bin/env python3
"""Reinforcement Learning Inference Script"""
import os
import gymnasium as gym
from stable_baselines3 import PPO

MODEL_PATH = "/model"
OUTPUT_PATH = "/tmp/artha/output"

def main():
    env = gym.make("CartPole-v1", render_mode="human")
    model = PPO.load(os.path.join(MODEL_PATH, "final_model"))
    
    obs, _ = env.reset()
    total_reward = 0
    for _ in range(1000):
        action, _ = model.predict(obs, deterministic=True)
        obs, reward, terminated, truncated, _ = env.step(action)
        total_reward += reward
        if terminated or truncated:
            break
    
    print(f"✅ Inference complete: Total reward = {total_reward}")

if __name__ == "__main__":
    main()

