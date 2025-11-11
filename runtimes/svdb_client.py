#!/usr/bin/env python3
"""
SVDB Client Library for Runtime Containers
Handles mounting, downloading, and accessing SVDB content
"""

import os
import requests
import json
from typing import Optional, List
from pathlib import Path

class SVDBClient:
    def __init__(self, api_url: str = None):
        self.api_url = api_url or os.environ.get("SVDB_API_URL", "http://localhost:8080")
    
    def download(self, cid_uri: str, output_path: str, chunk_size: int = 8192) -> bool:
        """Download content from SVDB by CID"""
        cid = cid_uri.replace("artha://", "")
        url = f"{self.api_url}/svdb/download/{cid}"
        
        try:
            response = requests.get(url, stream=True, timeout=300)
            response.raise_for_status()
            
            os.makedirs(os.path.dirname(output_path) or ".", exist_ok=True)
            with open(output_path, 'wb') as f:
                for chunk in response.iter_content(chunk_size=chunk_size):
                    f.write(chunk)
            
            return True
        except Exception as e:
            print(f"❌ SVDB download failed: {e}")
            return False
    
    def upload(self, file_path: str, replicas: int = 5, months: int = 12) -> Optional[str]:
        """Upload file to SVDB and return CID"""
        url = f"{self.api_url}/svdb/upload"
        
        try:
            with open(file_path, 'rb') as f:
                files = {'file': (os.path.basename(file_path), f)}
                data = {
                    'replicas': str(replicas),
                    'months': str(months),
                }
                response = requests.post(url, files=files, data=data, timeout=300)
                response.raise_for_status()
                
                result = response.json()
                cid = result.get('cid', '')
                return f"artha://{cid}" if cid else None
        except Exception as e:
            print(f"❌ SVDB upload failed: {e}")
            return None
    
    def mount(self, cid_uri: str, mount_point: str) -> bool:
        """Mount SVDB content to local path (downloads if not using FUSE)"""
        # For now: download to mount point
        # In production: use SVDB FUSE mount
        return self.download(cid_uri, mount_point)
    
    def info(self, cid_uri: str) -> Optional[dict]:
        """Get metadata about SVDB content"""
        cid = cid_uri.replace("artha://", "")
        url = f"{self.api_url}/svdb/info/{cid}"
        
        try:
            response = requests.get(url, timeout=10)
            response.raise_for_status()
            return response.json()
        except Exception as e:
            print(f"❌ SVDB info failed: {e}")
            return None
    
    def list_manifest(self, manifest_cid: str) -> List[str]:
        """List all chunks in a manifest"""
        # Download manifest
        manifest_path = "/tmp/manifest.json"
        if not self.download(f"artha://{manifest_cid}", manifest_path):
            return []
        
        try:
            with open(manifest_path, 'r') as f:
                manifest = json.load(f)
            return manifest.get('chunks', [])
        except:
            return []

# Global instance
svdb = SVDBClient()

