#!/usr/bin/env python3
"""ComfyUI Wrapper for Stable Diffusion Runtime"""

import os
import sys
import json
import requests
from typing import Dict

# Add ComfyUI API wrapper
try:
    import comfy
    from comfy import api
except ImportError:
    print("⚠️  ComfyUI not installed, using direct API calls")

COMFYUI_URL = os.environ.get("COMFYUI_URL", "http://localhost:8188")
JOB_ID = os.environ.get("ARTHA_JOB_ID", "sd-unknown")

def generate_image(prompt: str, negative_prompt: str = "", steps: int = 20) -> str:
    """Generate image using ComfyUI"""
    try:
        # ComfyUI workflow JSON
        workflow = {
            "prompt": prompt,
            "negative_prompt": negative_prompt,
            "steps": steps,
        }
        
        # Submit to ComfyUI API
        resp = requests.post(
            f"{COMFYUI_URL}/prompt",
            json=workflow,
            timeout=300,
        )
        
        if resp.status_code == 200:
            result = resp.json()
            return result.get("output_path", "/tmp/generated.png")
        else:
            return None
    except Exception as e:
        print(f"ComfyUI error: {e}")
        return None

def main():
    prompt = os.environ.get("PROMPT", "a beautiful landscape")
    
    print(f"🎨 ComfyUI Generation - Job: {JOB_ID}")
    print(f"   Prompt: {prompt}")
    
    output_path = generate_image(prompt)
    
    if output_path:
        print(f"✅ Image generated: {output_path}")
        
        # Upload to SVDB
        sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
        try:
            from svdb_client import svdb
            cid = svdb.upload(output_path, replicas=5, months=12)
            print(f"   Uploaded to SVDB: {cid}")
        except:
            pass
    else:
        print("❌ Generation failed")

if __name__ == "__main__":
    main()

