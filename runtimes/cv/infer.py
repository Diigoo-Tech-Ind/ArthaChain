#!/usr/bin/env python3
"""Computer Vision Inference Script"""
import os
from ultralytics import YOLO
import cv2

MODEL_PATH = "/model"
OUTPUT_PATH = "/tmp/artha/output"

def main():
    model = YOLO(os.path.join(MODEL_PATH, "model.pt"))
    # Load image from input
    # For demo: dummy image
    results = model.predict("https://ultralytics.com/images/bus.jpg", save=True)
    print(f"✅ Inference complete: {len(results)} detections")

if __name__ == "__main__":
    main()

