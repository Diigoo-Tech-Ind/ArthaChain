#!/usr/bin/env python3
"""Stable Diffusion Inference Script"""
import os
from diffusers import StableDiffusionPipeline
import torch

MODEL_PATH = "/model"
OUTPUT_PATH = "/tmp/artha/output"

def main():
    pipe = StableDiffusionPipeline.from_pretrained(
        MODEL_PATH if os.path.exists(MODEL_PATH) else "runwayml/stable-diffusion-v1-5",
        torch_dtype=torch.float16 if torch.cuda.is_available() else torch.float32
    )
    pipe = pipe.to("cuda" if torch.cuda.is_available() else "cpu")
    
    prompt = os.environ.get("PROMPT", "a beautiful landscape")
    image = pipe(prompt).images[0]
    
    os.makedirs(OUTPUT_PATH, exist_ok=True)
    image.save(os.path.join(OUTPUT_PATH, "output.png"))
    print(f"✅ Generated image: {OUTPUT_PATH}/output.png")

if __name__ == "__main__":
    main()

