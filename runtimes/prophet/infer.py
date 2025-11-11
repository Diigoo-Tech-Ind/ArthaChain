#!/usr/bin/env python3
"""Prophet Inference Script"""
import os
import pickle
import pandas as pd

MODEL_PATH = "/model"
OUTPUT_PATH = "/tmp/artha/output"

def main():
    with open(os.path.join(MODEL_PATH, "model.pkl"), 'rb') as f:
        model = pickle.load(f)
    
    # Forecast next 30 days
    future = model.make_future_dataframe(periods=30)
    forecast = model.predict(future)
    
    os.makedirs(OUTPUT_PATH, exist_ok=True)
    forecast.to_csv(os.path.join(OUTPUT_PATH, "forecast.csv"), index=False)
    
    print(f"✅ Forecast complete: {len(forecast)} predictions")

if __name__ == "__main__":
    main()

