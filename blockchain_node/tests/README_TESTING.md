# ArthaChain SVDB Testing Suite

Complete testing infrastructure for validating all claims about the SVDB system.

---

## 📋 Test Coverage Overview

### ✅ **Integration Tests** (`integration_test_runner.sh`)
- **Purpose**: Validate end-to-end functionality with real multi-node infrastructure
- **Duration**: ~5-10 minutes
- **Requirements**: 
  - Local blockchain (Ganache or similar)
  - Forge (for contract deployment)
  - 5 GB free disk space
  - 8 GB RAM

**Test Cases:**
1. ✅ **Upload & Replicate** - 100MB file to 5 nodes
2. ✅ **Erasure Coding & Repair** - 1GB file with simulated node failure
3. ✅ **Proof Challenge Cycle** - 10 epochs of proof generation
4. ✅ **Marketplace Integration** - Provider listing and querying
5. ✅ **One-Click AI Training** - Job submission and monitoring

### ✅ **Performance Benchmarks** (`benchmark_suite.sh`)
- **Purpose**: Validate all performance claims with real measurements
- **Duration**: ~15-20 minutes
- **Requirements**:
  - CUDA-capable GPU (optional, for PoRep seal benchmark)
  - 10 GB free disk space
  - High-speed disk (NVMe recommended)

**Benchmarks:**
1. ✅ **Upload Throughput** - Target: ≥2 Gbps
2. ✅ **Download Latency** - Target: <150ms first byte
3. ✅ **Download Throughput** - Target: <1.5s for 100MB
4. ✅ **Proof Verification** - Target: ≤200ms per proof
5. ✅ **GPU PoRep Seal** - Target: ~28s on A100
6. ✅ **Concurrent Uploads** - Target: ≥10 parallel uploads
7. ✅ **CID Computation** - Target: >1 GB/s

### ✅ **Audit Preparation** (`AUDIT_PREPARATION.md`)
- Comprehensive security review of all 8 smart contracts
- Known issues and mitigation strategies
- Gas optimization opportunities
- Deployment checklist

---

## 🚀 Quick Start

### 1. Run Integration Tests
```bash
cd /Users/sainathtangallapalli/blockchain/ArthaChain/blockchain_node/tests
./integration_test_runner.sh
```

**Expected Output:**
```
🚀 ArthaChain SVDB Integration Test Suite
==========================================

✓ Test directories created
✓ Ganache started on port 8545
✓ Contracts deployed
  DealMarket: 0x5FbDB...
  OfferBook: 0xe7f17...
  PoRep: 0x9fE46...
✓ Nodes are ready

Test 1: Upload 100MB file with 5 replicas
  Uploaded CID: artha://bafy2bzacea...
  ✓ Node 1 has the file
  ✓ Node 2 has the file
  ✓ Node 3 has the file
  ✓ Node 4 has the file
  ✓ Node 5 has the file
✓ Test 1 PASSED

...

==========================================
Test Results Summary
==========================================
Passed: 5
Failed: 0
==========================================

🎉 ALL TESTS PASSED
```

### 2. Run Performance Benchmarks
```bash
cd /Users/sainathtangallapalli/blockchain/ArthaChain/blockchain_node/tests
./benchmark_suite.sh
```

**Expected Output:**
```
═══════════════════════════════════════════════
  ArthaChain SVDB Performance Benchmark Suite
═══════════════════════════════════════════════

✓ Node is running

Benchmark 1: Upload Throughput
Target: ≥ 2 Gbps (250 MB/s)
  Result: 2.34 Gbps (292.5 MB/s)
  ✓ PASS

Benchmark 2: Download First Byte Latency
Target: < 150 ms
  Result: 98.5 ms (avg of 10 samples)
  ✓ PASS

...

═══════════════════════════════════════════════
  Benchmark Complete
═══════════════════════════════════════════════

Results saved to: benchmark_results/benchmark_20251102_143022.json
Pass Rate: 100.0% (7/7)

Summary:
  upload_throughput_gbps: 2.34 Gbps (PASS)
  download_latency_ms: 98.5 ms (PASS)
  download_100mb_seconds: 1.12 s (PASS)
  proof_verification_ms: 145.2 ms (PASS)
  gpu_porep_seal_seconds: 31.5 s (PASS)
  concurrent_uploads_seconds: 8.7 s (PASS)
  cid_computation_gbps: 1.87 GB/s (PASS)
```

---

## 🔧 Prerequisites

### System Requirements
```bash
# Required
- Rust 1.70+
- Cargo
- curl, jq, bc
- 16 GB RAM
- 20 GB free disk space

# Optional (for full benchmark suite)
- CUDA 12+ (for GPU proving)
- NVIDIA GPU (A100/H100 for optimal results)
- Forge (Foundry) for contract deployment
- Ganache or local Ethereum node
```

### Install Dependencies

#### macOS
```bash
brew install jq bc curl
brew install --cask ganache  # Optional
curl -L https://foundry.paradigm.xyz | bash
foundryup
```

#### Linux (Ubuntu/Debian)
```bash
sudo apt update
sudo apt install -y jq bc curl
# Install Foundry
curl -L https://foundry.paradigm.xyz | bash
foundryup
# Install Ganache (optional)
npm install -g ganache
```

### Build the Project
```bash
cd /Users/sainathtangallapalli/blockchain/ArthaChain/blockchain_node
cargo build --release
cargo build --release --bin artha_prover_cuda
```

---

## 📊 Understanding Test Results

### Integration Test Status
- ✅ **PASS** - Test completed successfully
- ✗ **FAIL** - Test failed, check logs in `test_logs/`
- ⚠ **WARN** - Test passed with warnings

### Benchmark Status
- ✅ **PASS** - Met or exceeded target
- ✗ **FAIL** - Below target performance
- ⚠ **WARN** - Close to target but not optimal
- 🔄 **SKIP** - Test skipped (missing dependencies)

### Results Location
```
blockchain_node/
├── test_logs/           # Integration test logs
│   ├── ganache.log
│   ├── node1.log
│   ├── node2.log
│   └── ...
├── test_data/           # Test files and artifacts
│   ├── contracts.env    # Deployed contract addresses
│   └── node*/           # Per-node storage
└── benchmark_results/   # Benchmark JSON reports
    └── benchmark_TIMESTAMP.json
```

---

## 🐛 Troubleshooting

### Integration Tests

#### "Ganache not found"
```bash
# Install Ganache
npm install -g ganache

# Or use an existing chain
# Edit integration_test_runner.sh and set:
# CHAIN_URL="http://your-chain:8545"
```

#### "Nodes failed to start"
- Check if ports 3000-3004 and 9000-9004 are free
- Check logs: `cat test_logs/node1.log`
- Ensure you have enough disk space

#### "Contract deployment failed"
```bash
# Verify Forge is installed
forge --version

# Build contracts
cd ../contracts
forge build
```

### Performance Benchmarks

#### "GPU prover not found"
```bash
# Build the GPU prover
cargo build --release --bin artha_prover_cuda

# If you don't have a GPU, the test will be skipped
# The prover will fallback to CPU mode
```

#### "Upload throughput below target"
- Check disk I/O: `iostat -x 1`
- Ensure you're using NVMe or SSD (not HDD)
- Check available RAM: `free -h`
- Close other resource-heavy applications

#### "Download latency high"
- Check network latency: `ping localhost`
- Disable VPN/proxy
- Check system load: `top` or `htop`

---

## 📈 CI/CD Integration

### GitHub Actions Example
```yaml
name: SVDB Integration Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Install dependencies
        run: |
          sudo apt install -y jq bc curl
          npm install -g ganache
          curl -L https://foundry.paradigm.xyz | bash
          source ~/.bashrc && foundryup
      
      - name: Build project
        run: |
          cd blockchain_node
          cargo build --release
      
      - name: Run integration tests
        run: |
          cd blockchain_node/tests
          ./integration_test_runner.sh
      
      - name: Upload test logs
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: test-logs
          path: blockchain_node/test_logs/
```

---

## 🔍 Manual Testing

If you want to test individual components manually:

### Test 1: Upload a File
```bash
# Start a node
ARTHA_API_PORT=3000 ARTHA_ROLE_SP=true ./target/release/arthachain_node

# Upload a file
curl -X POST http://localhost:3000/svdb/upload \
  -F "file=@myfile.dat" \
  -H "X-Artha-Replicas: 3"

# Response:
# {"cid": "artha://bafy2bzacea..."}
```

### Test 2: Generate a Proof
```bash
# Generate Merkle proof for index 5
curl -X POST http://localhost:3000/svdb/proofs/branch \
  -H "Content-Type: application/json" \
  -d '{"cid": "artha://bafy2bzacea...", "index": 5}'

# Response:
# {
#   "root": "0x123...",
#   "leaf": "0xabc...",
#   "branch": ["0xdef...", "0x456..."],
#   "index": 5
# }
```

### Test 3: Query Marketplace
```bash
# Get active providers
curl "http://localhost:3000/svdb/marketplace/providers?rpcUrl=http://localhost:8545&contract=0x..."

# Response:
# {
#   "providers": [
#     {"address": "0xabc...", "region": "us-west", "price": "1000000000000000"}
#   ]
# }
```

---

## 📝 Test Maintenance

### Adding New Tests

1. **Integration Test**
   - Edit `integration_test_runner.sh`
   - Add a new function: `test_your_feature()`
   - Call it in `main()`

2. **Benchmark**
   - Edit `benchmark_suite.sh`
   - Add a new function: `benchmark_your_metric()`
   - Call `add_result()` with results

### Updating Test Expectations

If performance targets change:
```bash
# Edit benchmark_suite.sh
# Update the target values in comments and comparisons
# Example: Change "Target: ≥ 2 Gbps" to "Target: ≥ 3 Gbps"
```

---

## 🎯 Test Goals

### Current Status
- ✅ 5 end-to-end integration tests implemented
- ✅ 7 performance benchmarks implemented
- ✅ Audit preparation document complete
- ⏳ CI/CD pipeline (to be configured)
- ⏳ Fuzz testing (to be added)

### Future Enhancements
- [ ] Chaos engineering tests (random node failures)
- [ ] 30-day long-running stress test
- [ ] Cross-region latency tests
- [ ] Smart contract fuzz testing with Echidna
- [ ] Formal verification with Certora

---

## 📞 Support

**Issues?** Report bugs or ask questions:
- GitHub Issues: https://github.com/arthachain/arthachain/issues
- Discord: #svdb-testing
- Email: dev@arthachain.online

---

**Last Updated:** 2025-11-02  
**Version:** 1.0.0

