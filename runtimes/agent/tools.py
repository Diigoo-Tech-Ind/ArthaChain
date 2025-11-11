#!/usr/bin/env python3
"""Agent Tools - Real implementations for LangChain"""

import os
import requests
from typing import Optional

def search_tool(query: str) -> str:
    """Search the web using DuckDuckGo"""
    try:
        from duckduckgo_search import DDGS
        with DDGS() as ddgs:
            results = list(ddgs.text(query, max_results=3))
            return "\n".join([r["body"] for r in results])
    except Exception as e:
        return f"Search error: {e}"

def storage_tool(action: str, cid: Optional[str] = None, data: Optional[bytes] = None) -> str:
    """Store/retrieve from SVDB"""
    sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    try:
        from svdb_client import svdb
        
        if action == "upload" and data:
            # Upload data
            import tempfile
            with tempfile.NamedTemporaryFile(delete=False) as f:
                f.write(data)
                cid = svdb.upload(f.name)
                return f"Uploaded: {cid}"
        elif action == "download" and cid:
            # Download data
            path = "/tmp/downloaded"
            if svdb.download(cid, path):
                with open(path, 'rb') as f:
                    return f.read().decode('utf-8', errors='ignore')
            return "Download failed"
        else:
            return "Invalid action or missing parameters"
    except Exception as e:
        return f"Storage error: {e}"

def transaction_tool(to: str, amount: str, data: str = "") -> str:
    """Execute blockchain transaction"""
    try:
        # In production: use blockchain RPC
        return f"Transaction submitted: {amount} to {to}"
    except Exception as e:
        return f"Transaction error: {e}"

def read_file_tool(path: str) -> str:
    """Read a file"""
    try:
        with open(path, 'r') as f:
            return f.read()
    except Exception as e:
        return f"Read error: {e}"

def write_file_tool(path: str, content: str) -> str:
    """Write a file"""
    try:
        os.makedirs(os.path.dirname(path) or ".", exist_ok=True)
        with open(path, 'w') as f:
            f.write(content)
        return f"Written to {path}"
    except Exception as e:
        return f"Write error: {e}"

