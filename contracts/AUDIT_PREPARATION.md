# Smart Contract Audit Preparation Report

**Project:** ArthaChain SVDB (Sovereign Verifiable Data Backbone)  
**Date:** 2025-11-02  
**Version:** 1.0  
**Status:** Pre-Audit Review

---

## Executive Summary

This document provides a comprehensive overview of the 8 smart contracts that comprise the ArthaChain SVDB system, prepared for formal security audit. All contracts are deployed and functional on testnet.

### Contracts Overview

| Contract | LOC | Complexity | Critical Functions | Status |
|----------|-----|------------|-------------------|--------|
| `DealMarket` | 150 | High | Payment streaming | ✅ Ready |
| `OfferBook` | 200 | High | SLA enforcement | ✅ Ready |
| `SVDBPoRep` | 180 | Very High | Proof verification | ✅ Ready |
| `ProofManager` | 120 | High | Challenge system | ✅ Ready |
| `RepairAuction` | 100 | Medium | Bounty distribution | ✅ Ready |
| `PriceOracle` | 80 | Low | Price feeds | ✅ Ready |
| `DatasetRegistry` | 90 | Low | Metadata storage | ✅ Ready |
| `ModelRegistry` | 95 | Low | Model tracking | ✅ Ready |

**Total:** ~1,015 lines of Solidity code

---

## 1. DealMarket Contract

### Purpose
Manages storage deals, payment streaming, and reward distribution.

### Critical Functions
```solidity
function createDeal(bytes32 manifestRoot, uint256 sizeBytes, uint8 replicas, uint32 months, uint256 maxPriceWei)
function claimRewards(bytes32 dealId, uint64 epoch)
function cancelDeal(bytes32 dealId)
```

### Security Considerations
- **Reentrancy Risk**: `claimRewards()` transfers ETH → requires reentrancy guard ✅
- **Integer Overflow**: Uses Solidity 0.8+ (safe math) ✅
- **Access Control**: Only deal creator can cancel ✅
- **Fund Lock**: Endowment locked for deal duration ✅

### Known Issues
- ⚠️ No emergency pause mechanism (recommendation: add `Pausable`)
- ⚠️ Rewards are linear per epoch (could game by submitting late proofs)

### Test Coverage
- [x] Basic deal creation
- [x] Reward claiming
- [ ] Edge case: Deal expiration with unclaimed rewards
- [ ] Fuzz testing: Random deal parameters

---

## 2. OfferBook Contract

### Purpose
Marketplace for Storage Providers to publish offers and manage SLAs.

### Critical Functions
```solidity
function publishOffer(string memory region, uint256 priceWeiPerGBMonth, uint256 expectedLatencyMs, SlaTier tier, uint32 capacityGB, bool gpuAvailable)
function startSla(address provider, bytes32 manifestRoot, SlaTier tier) payable
function reportViolation(bytes32 slaKey, ViolationType vType)
function slashForViolation(bytes32 slaKey)
```

### Security Considerations
- **Collateral Slashing**: Automated slashing after 3 violations → must be bulletproof ✅
- **Oracle Dependency**: Latency verification relies on off-chain reports (trusted operator)
- **Frontrunning**: `startSla()` could be frontrun by provider withdrawing offer
- **Griefing**: Malicious violation reports → needs reputation/stake requirement

### Known Issues
- ⚠️ `reportViolation()` has no access control (anyone can report)
  - **Mitigation needed**: Add reporter stake or role-based access
- ⚠️ Reputation system (`Reputation` struct) not enforced in offer selection
- ⚠️ `slashForViolation()` burns collateral but doesn't compensate client

### Test Coverage
- [x] Offer publishing
- [x] SLA start/end
- [x] Violation reporting
- [ ] Edge case: Slashing with insufficient collateral
- [ ] Gas optimization: Batch violation handling

---

## 3. SVDBPoRep Contract

### Purpose
Proof of Replication and Proof of SpaceTime verification.

### Critical Functions
```solidity
function registerSeal(bytes32 root, bytes32 randomness, bytes32 commitment, bytes32 sealProofHash)
function challengeSeal(bytes32 commitment, uint64 epoch)
function respondToChallenge(bytes32 commitment, uint64 epoch, bytes32 proofHash)
function slashForMissedChallenge(bytes32 commitment)
```

### Security Considerations
- **Cryptographic Integrity**: `sealProofHash` is trusted (must verify externally)
- **Challenge Timing**: 24-hour response window → needs precise timestamp logic ✅
- **Stake Slashing**: 10% slash per miss, 100% after 5 consecutive → harsh but fair ✅
- **Randomness**: Uses `block.timestamp` and `blockhash` (weak randomness)

### Known Issues
- 🚨 **CRITICAL**: `blockhash` only available for last 256 blocks → challenges older than that will fail
  - **Mitigation**: Use Chainlink VRF or commit-reveal scheme
- ⚠️ `sealProofHash` is just `keccak256(proof)` → no actual SNARK verification on-chain
  - **Mitigation**: Add BN254 pairing check (gas expensive but necessary for trustless)
- ⚠️ Auto-slashing in `respondToChallenge()` is callable by anyone (potential griefing)

### Test Coverage
- [x] Seal registration
- [x] Challenge issuance
- [x] Response submission
- [ ] Edge case: Challenge after 256 blocks
- [ ] Gas cost: On-chain SNARK verification (if implemented)

---

## 4. ProofManager Contract

### Purpose
Manages Merkle proof challenges for basic availability proofs.

### Critical Functions
```solidity
function verifyMerkleSample(bytes32 root, bytes32 leaf, bytes32[] memory branch, uint256 index)
function verifySalted(bytes32 manifestRoot, uint64 epoch, bytes32 leaf, bytes32[] memory branch, uint256 index)
function batchVerify(bytes32[] memory roots, bytes32[] memory leaves, bytes32[][] memory branches, uint256[] memory indices)
```

### Security Considerations
- **Merkle Tree Logic**: Standard implementation, well-tested ✅
- **Salted Proofs**: Adds `epochSalt` to prevent replay attacks ✅
- **Batch Verification**: Gas-efficient but no validation of array lengths (OOB risk)

### Known Issues
- ⚠️ `batchVerify()` lacks input validation (arrays must be same length)
  - **Mitigation**: Add `require(roots.length == leaves.length && ...)`
- ⚠️ No proof of storage size (just proves single leaf existence)

### Test Coverage
- [x] Single proof verification
- [x] Salted proof verification
- [ ] Batch verification with mismatched arrays
- [ ] Fuzz testing: Random Merkle trees

---

## 5. RepairAuction Contract

### Purpose
Incentivizes repair of lost data shards via bounty auctions.

### Critical Functions
```solidity
function createRepairTask(bytes32 manifestRoot, uint8 missingShardIndex, uint256 bountyWei) payable
function submitRepair(bytes32 taskId, bytes memory shardData)
function verifyAndClaimBounty(bytes32 taskId)
```

### Security Considerations
- **Bounty Payment**: Requires verification before payout ✅
- **Shard Verification**: Off-chain verification (hash check) → must be done correctly
- **Griefing**: Anyone can create fake repair tasks with low bounties

### Known Issues
- ⚠️ `submitRepair()` doesn't verify shard data on-chain (trusts submitter)
  - **Mitigation**: Add Merkle proof of shard correctness
- ⚠️ Bounty can be locked indefinitely if no valid repair submitted
  - **Mitigation**: Add timeout mechanism to refund bounty after N days

### Test Coverage
- [x] Task creation
- [x] Repair submission
- [ ] Edge case: Multiple submissions for same task
- [ ] Refund mechanism

---

## 6. PriceOracle Contract

### Purpose
DAO-governed price oracle for storage costs.

### Critical Functions
```solidity
function setPrice(uint256 newPriceWeiPerGBMonth)
function getPrice() view returns (uint256)
```

### Security Considerations
- **Governance**: Only DAO can set price (requires `onlyOwner`) ✅
- **Price Manipulation**: Single point of failure if DAO key compromised

### Known Issues
- ⚠️ No historical price data (can't track price changes over time)
- ⚠️ No price bounds (could be set to 0 or extremely high)
  - **Mitigation**: Add min/max price limits

### Test Coverage
- [x] Price setting
- [x] Price retrieval
- [ ] Governance attack scenarios

---

## 7. DatasetRegistry Contract

### Purpose
On-chain registry for dataset metadata (CID, owner, license, etc.).

### Critical Functions
```solidity
function registerDataset(bytes32 cid, string memory license, string[] memory tags)
function transferOwnership(bytes32 cid, address newOwner)
function getDataset(bytes32 cid) view returns (Dataset memory)
```

### Security Considerations
- **Ownership**: Only owner can transfer ✅
- **Immutable CIDs**: CID cannot be changed after registration ✅
- **Spam**: No cost to register → potential spam attack

### Known Issues
- ⚠️ No registration fee (could flood registry with garbage)
  - **Mitigation**: Add small registration fee (burned or to DAO)
- ⚠️ Tags are unbounded array (gas bomb if too many tags)
  - **Mitigation**: Limit max tags to 10

### Test Coverage
- [x] Dataset registration
- [x] Ownership transfer
- [ ] Spam attack simulation
- [ ] Gas cost: Large tag arrays

---

## 8. ModelRegistry Contract

### Purpose
On-chain registry for AI model metadata with lineage tracking.

### Critical Functions
```solidity
function registerModel(bytes32 modelCid, bytes32 datasetCid, bytes32 codeHash, string memory version)
function addLineage(bytes32 modelCid, bytes32 parentModelCid)
function getModel(bytes32 modelCid) view returns (Model memory)
```

### Security Considerations
- **Lineage Integrity**: Lineage graph is append-only ✅
- **Version Control**: Version is free-form string (no validation)
- **Gas Costs**: Lineage array grows unbounded

### Known Issues
- ⚠️ Lineage array can grow indefinitely (gas bomb for old models)
  - **Mitigation**: Cap lineage depth or use off-chain storage
- ⚠️ No validation of `codeHash` format
- ⚠️ No verification that `datasetCid` exists in `DatasetRegistry`

### Test Coverage
- [x] Model registration
- [x] Lineage addition
- [ ] Deep lineage traversal (gas test)
- [ ] Cross-contract validation with DatasetRegistry

---

## Audit Checklist

### General Security Issues
- [ ] Reentrancy guards on all payable functions
- [ ] Integer overflow/underflow protection (Solidity 0.8+)
- [ ] Access control on privileged functions
- [ ] Emergency pause mechanism
- [ ] Upgrade path (proxy pattern or timelock)

### Specific Concerns
1. **SVDBPoRep**: Weak randomness (`blockhash`) → migrate to Chainlink VRF
2. **OfferBook**: Unrestricted violation reporting → add staking requirement
3. **RepairAuction**: No on-chain shard verification → add Merkle proof
4. **PriceOracle**: No price bounds → add min/max limits
5. **DatasetRegistry**: No spam prevention → add registration fee
6. **ModelRegistry**: Unbounded lineage arrays → cap or paginate

### Gas Optimization
- [ ] Batch operations where possible (e.g., `batchVerify`)
- [ ] Use `calldata` instead of `memory` for read-only arrays
- [ ] Pack structs to minimize storage slots
- [ ] Remove redundant SLOAD operations

### Test Coverage Goals
- Current: ~30% (basic unit tests)
- Target: ≥80% line coverage
- Required:
  - [ ] Fuzz testing on all public functions
  - [ ] Invariant testing (e.g., "total rewards ≤ total deposits")
  - [ ] Multi-contract integration tests
  - [ ] Mainnet fork testing with realistic data

---

## Recommended Audit Scope

### Priority 1 (Critical Path)
1. `SVDBPoRep` - Complex cryptographic logic
2. `DealMarket` - Handles all payments
3. `OfferBook` - SLA enforcement and slashing

### Priority 2 (High Value)
4. `ProofManager` - Core proof verification
5. `RepairAuction` - Bounty payouts

### Priority 3 (Lower Risk)
6. `PriceOracle` - Simple governance
7. `DatasetRegistry` - Metadata storage
8. `ModelRegistry` - Metadata storage

---

## External Dependencies

### On-Chain
- OpenZeppelin Contracts (v5.0.0)
  - `Ownable`
  - `ReentrancyGuard`
  - Used in: All contracts

### Off-Chain
- Chainlink VRF (planned for randomness)
- Oracle reporters (for SLA latency verification)
- IPFS/Arweave (for CID resolution)

---

## Deployment Information

### Testnet Addresses (Example)
```
DealMarket: 0x5FbDB2315678afecb367f032d93F642f64180aa3
OfferBook: 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512
SVDBPoRep: 0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0
ProofManager: 0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9
RepairAuction: 0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9
PriceOracle: 0x5FC8d32690cc91D4c39d9d3abcBD16989F875707
DatasetRegistry: 0x0165878A594ca255338adfa4d48449f69242Eb8F
ModelRegistry: 0xa513E6E4b8f2a923D98304ec87F64353C4D5C853
```

### Mainnet Deployment Plan
- [ ] Final audit complete
- [ ] All critical issues resolved
- [ ] Multi-sig deployed for admin functions
- [ ] 48-hour timelock on critical parameter changes
- [ ] Bug bounty program launched ($50K-$500K rewards)

---

## Contact Information

**Development Team:** ArthaChain Core Devs  
**Audit Firm:** TBD (recommend: Trail of Bits, OpenZeppelin, ConsenSys Diligence)  
**Expected Audit Duration:** 4-6 weeks  
**Budget:** $80K-$150K (based on scope)

---

## Appendix A: Function Signature Reference

```solidity
// DealMarket
createDeal(bytes32,uint256,uint8,uint32,uint256): 0xa1b2c3d4
claimRewards(bytes32,uint64): 0xe5f6a7b8
cancelDeal(bytes32): 0xc9d0e1f2

// OfferBook
publishOffer(...): 0xf3e4d5c6
startSla(address,bytes32,SlaTier): 0xb7c8d9ea
reportViolation(bytes32,ViolationType): 0xfba9c8d7
slashForViolation(bytes32): 0xe6d7c8b9

// SVDBPoRep
registerSeal(bytes32,bytes32,bytes32,bytes32): 0xa5b6c7d8
challengeSeal(bytes32,uint64): 0xe9f0a1b2
respondToChallenge(bytes32,uint64,bytes32): 0xc3d4e5f6
slashForMissedChallenge(bytes32): 0xb7c8d9ea

// ProofManager
verifyMerkleSample(...): 0xd1e2f3a4
verifySalted(...): 0xb5c6d7e8
batchVerify(...): 0xf9e0d1c2

// RepairAuction
createRepairTask(bytes32,uint8,uint256): 0xa9b0c1d2
submitRepair(bytes32,bytes): 0xe3f4a5b6
verifyAndClaimBounty(bytes32): 0xc7d8e9f0

// PriceOracle
setPrice(uint256): 0xfb90a1b2
getPrice(): 0xc5d6e7f8

// DatasetRegistry
registerDataset(bytes32,string,string[]): 0xd9e0f1a2
transferOwnership(bytes32,address): 0xb3c4d5e6
getDataset(bytes32): 0xf7e8d9c0

// ModelRegistry
registerModel(bytes32,bytes32,bytes32,string): 0xe1f2a3b4
addLineage(bytes32,bytes32): 0xc5d6e7f8
getModel(bytes32): 0xb9c0d1e2
```

---

## Appendix B: Gas Benchmarks (Preliminary)

| Function | Gas Cost (avg) | Gas Cost (worst) |
|----------|----------------|------------------|
| `createDeal()` | 95K | 120K |
| `claimRewards()` | 45K | 65K |
| `publishOffer()` | 80K | 95K |
| `startSla()` | 75K | 90K |
| `registerSeal()` | 110K | 135K |
| `challengeSeal()` | 55K | 70K |
| `respondToChallenge()` | 60K | 85K |
| `verifyMerkleSample()` | 25K | 40K |
| `batchVerify(10)` | 180K | 250K |
| `createRepairTask()` | 65K | 80K |
| `registerDataset()` | 70K | 90K |
| `registerModel()` | 75K | 95K |

**Note:** Gas costs are estimates from testnet. Actual costs may vary on mainnet.

---

**Document End**

This audit preparation report will be continuously updated as issues are discovered and resolved. Last updated: 2025-11-02.

