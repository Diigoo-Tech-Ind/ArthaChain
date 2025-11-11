#!/usr/bin/env python3
"""Recommendation Inference Script"""
import os
import pickle
import numpy as np

MODEL_PATH = "/model"
OUTPUT_PATH = "/tmp/artha/output"

def main():
    with open(os.path.join(MODEL_PATH, "model.pkl"), 'rb') as f:
        model = pickle.load(f)
    
    # Predict for user
    user_id = 0
    n_items = 10
    scores = model.predict(user_id, np.arange(n_items))
    top_items = np.argsort(-scores)[:5]
    
    print(f"✅ Recommendations: {top_items}")

if __name__ == "__main__":
    main()

