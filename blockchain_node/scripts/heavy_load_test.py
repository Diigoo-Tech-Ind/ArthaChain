#!/usr/bin/env python3
"""
Heavy Load Test for ArthaChain
- Deploy 5 smart contracts simultaneously
- Execute 100 transactions with 1-5 MB payload each
"""

import json
import time
import random
import hashlib
import requests
import string
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime

API_URL = "http://localhost:1900"

def log(message, level="INFO"):
    timestamp = datetime.now().strftime("%H:%M:%S.%f")[:-3]
    symbols = {"INFO": "ℹ️", "SUCCESS": "✅", "ERROR": "❌", "DEPLOY": "📦", "TX": "💫", "DATA": "📊"}
    print(f"[{timestamp}] {symbols.get(level, '•')} {message}")

def generate_large_data(size_mb):
    """Generate random data of specified size in MB"""
    size_bytes = int(size_mb * 1024 * 1024)
    # Generate random hex string
    data = ''.join(random.choices(string.hexdigits.lower(), k=size_bytes * 2))
    return data[:size_bytes * 2]  # Ensure exact size

def deploy_contract(contract_name, contract_type):
    """Deploy a smart contract"""
    # Generate contract bytecode (simulated)
    bytecode = hashlib.sha256(f"{contract_name}_{contract_type}".encode()).hexdigest() * 100
    
    contract_data = {
        "address": f"0x{hashlib.sha256(contract_name.encode()).hexdigest()[:40]}",
        "transactions": [{
            "from": "0x0000000000000000000000000000000000001111",
            "to": "0x0000000000000000000000000000000000000000",  # Contract creation
            "value": 0,
            "gas_price": 20000000000,
            "gas_limit": 5000000,
            "nonce": random.randint(0, 10000),
            "data": bytecode
        }]
    }
    
    try:
        resp = requests.post(
            f"{API_URL}/api/v1/transactions/submit",
            json=contract_data,
            timeout=30
        )
        if resp.status_code == 200:
            return {"name": contract_name, "type": contract_type, "success": True, "address": contract_data["address"]}
        return {"name": contract_name, "type": contract_type, "success": False, "error": resp.text}
    except Exception as e:
        return {"name": contract_name, "type": contract_type, "success": False, "error": str(e)}

def submit_large_transaction(tx_id, size_mb):
    """Submit a transaction with large data payload"""
    # Generate large data payload
    large_data = generate_large_data(size_mb)
    
    tx_data = {
        "transactions": [{
            "from": f"0x{hashlib.sha256(f'sender_{tx_id}'.encode()).hexdigest()[:40]}",
            "to": f"0x{hashlib.sha256(f'receiver_{tx_id}'.encode()).hexdigest()[:40]}",
            "value": random.randint(1, 1000) * 10**18,
            "gas_price": 20000000000,
            "gas_limit": 10000000,  # High gas for large data
            "nonce": random.randint(0, 10000),
            "data": large_data
        }]
    }
    
    start_time = time.time()
    try:
        resp = requests.post(
            f"{API_URL}/api/v1/transactions/submit",
            json=tx_data,
            timeout=120  # Long timeout for large data
        )
        elapsed = time.time() - start_time
        
        if resp.status_code == 200:
            return {
                "tx_id": tx_id,
                "size_mb": size_mb,
                "success": True,
                "time_seconds": elapsed,
                "data_size_bytes": len(large_data)
            }
        return {
            "tx_id": tx_id,
            "size_mb": size_mb,
            "success": False,
            "error": resp.text[:200],
            "time_seconds": elapsed
        }
    except Exception as e:
        return {
            "tx_id": tx_id,
            "size_mb": size_mb,
            "success": False,
            "error": str(e),
            "time_seconds": time.time() - start_time
        }

def get_blockchain_height():
    try:
        resp = requests.get(f"{API_URL}/api/v1/blockchain/height", timeout=5)
        return resp.json().get("height", 0)
    except:
        return 0

def main():
    print("\n" + "="*70)
    print("  🚀 ArthaChain Heavy Load Test")
    print("  📦 5 Smart Contracts + 100 Large Transactions (1-5 MB each)")
    print("="*70 + "\n")
    
    start_time = time.time()
    initial_height = get_blockchain_height()
    log(f"Initial block height: {initial_height}")
    
    # ==========================================
    # PHASE 1: Deploy 5 Smart Contracts Simultaneously
    # ==========================================
    print("\n" + "-"*50)
    print("  📦 PHASE 1: Deploying 5 Smart Contracts")
    print("-"*50)
    
    contracts = [
        ("ArthaToken", "ERC20"),
        ("ArthaNFT", "ERC721"),
        ("ArthaSwap", "DEX"),
        ("ArthaDAO", "Governance"),
        ("ArthaVault", "DeFi")
    ]
    
    deployed = []
    with ThreadPoolExecutor(max_workers=5) as executor:
        futures = {executor.submit(deploy_contract, name, ctype): (name, ctype) 
                   for name, ctype in contracts}
        
        for future in as_completed(futures):
            result = future.result()
            if result["success"]:
                log(f"Deployed {result['name']} ({result['type']}): {result['address'][:20]}...", "DEPLOY")
                deployed.append(result)
            else:
                log(f"Failed to deploy {result['name']}: {result.get('error', 'Unknown')}", "ERROR")
    
    log(f"Deployed {len(deployed)}/5 contracts", "SUCCESS")
    
    # ==========================================
    # PHASE 2: Submit 100 Large Transactions (1-5 MB each)
    # ==========================================
    print("\n" + "-"*50)
    print("  💫 PHASE 2: 100 Large Transactions (1-5 MB each)")
    print("-"*50)
    
    large_tx_results = []
    total_data_mb = 0
    successful_txs = 0
    failed_txs = 0
    
    # Submit in batches of 10
    for batch in range(10):
        log(f"Submitting batch {batch + 1}/10 (10 transactions)...", "TX")
        
        batch_results = []
        with ThreadPoolExecutor(max_workers=10) as executor:
            futures = {}
            for i in range(10):
                tx_id = batch * 10 + i + 1
                size_mb = random.uniform(1, 5)  # Random size between 1-5 MB
                futures[executor.submit(submit_large_transaction, tx_id, size_mb)] = tx_id
            
            for future in as_completed(futures):
                result = future.result()
                batch_results.append(result)
                
                if result["success"]:
                    successful_txs += 1
                    total_data_mb += result["size_mb"]
                else:
                    failed_txs += 1
        
        large_tx_results.extend(batch_results)
        
        # Progress update
        avg_time = sum(r["time_seconds"] for r in batch_results) / len(batch_results)
        log(f"Batch {batch + 1}: {sum(1 for r in batch_results if r['success'])}/10 success, avg time: {avg_time:.2f}s", 
            "SUCCESS" if sum(1 for r in batch_results if r["success"]) >= 5 else "ERROR")
    
    # ==========================================
    # FINAL SUMMARY
    # ==========================================
    end_time = time.time()
    final_height = get_blockchain_height()
    duration = end_time - start_time
    
    print("\n" + "="*70)
    print("  📊 HEAVY LOAD TEST SUMMARY")
    print("="*70)
    
    print(f"""
    ┌─────────────────────────────────────────┐
    │  SMART CONTRACTS                        │
    │  ├─ Deployed:          {len(deployed):>15}   │
    │  └─ Target:            {5:>15}   │
    ├─────────────────────────────────────────┤
    │  LARGE TRANSACTIONS (1-5 MB each)       │
    │  ├─ Successful:        {successful_txs:>15}   │
    │  ├─ Failed:            {failed_txs:>15}   │
    │  └─ Total Data:        {total_data_mb:>12.2f} MB   │
    ├─────────────────────────────────────────┤
    │  BLOCKCHAIN                             │
    │  ├─ Initial Height:    {initial_height:>15}   │
    │  ├─ Final Height:      {final_height:>15}   │
    │  └─ Blocks Produced:   {final_height - initial_height:>15}   │
    ├─────────────────────────────────────────┤
    │  PERFORMANCE                            │
    │  ├─ Duration:          {duration:>12.2f}s   │
    │  ├─ Avg TX Time:       {sum(r['time_seconds'] for r in large_tx_results)/len(large_tx_results) if large_tx_results else 0:>12.2f}s   │
    │  └─ Data Throughput:   {total_data_mb/duration*1024 if duration > 0 else 0:>10.2f} KB/s   │
    └─────────────────────────────────────────┘
    """)
    
    # Save results
    results = {
        "timestamp": datetime.now().isoformat(),
        "contracts_deployed": len(deployed),
        "large_transactions": {
            "successful": successful_txs,
            "failed": failed_txs,
            "total_data_mb": total_data_mb
        },
        "blockchain": {
            "initial_height": initial_height,
            "final_height": final_height,
            "blocks_produced": final_height - initial_height
        },
        "performance": {
            "duration_seconds": duration,
            "data_throughput_kbps": total_data_mb/duration*1024 if duration > 0 else 0
        }
    }
    
    with open("heavy_load_test_results.json", "w") as f:
        json.dump(results, f, indent=2)
    
    log("Results saved to heavy_load_test_results.json", "SUCCESS")
    print("\n" + "="*70)

if __name__ == "__main__":
    main()
