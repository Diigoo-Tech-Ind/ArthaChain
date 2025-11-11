// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/**
 * @title ProofRegistry v1 (ABI FROZEN)
 * @notice Registry for storage and compute proofs with job/deal linkage
 * @dev Namespace: ArthaProofID
 */
contract ProofRegistry {
    enum ProofType { StorageAvailability, StorageReplication, ComputeExecution, DataIntegrity }
    
    struct Proof {
        bytes32 proofId;
        ProofType proofType;
        bytes32 proverId;       // Node pubkey or DID
        bytes32 targetHash;     // CID, jobId, or dealId
        bytes32 proofHash;      // Hash of proof data
        bytes32 proofCid;       // SVDB CID for full proof
        uint64 timestamp;
        uint64 epoch;
        bool verified;
        bytes32 verifierId;     // Node that verified (if applicable)
    }

    mapping(bytes32 => Proof) public proofs;
    mapping(bytes32 => bytes32[]) public proverProofs;     // proverId => proofIds
    mapping(bytes32 => bytes32[]) public targetProofs;     // targetHash => proofIds
    mapping(uint64 => bytes32[]) public epochProofs;       // epoch => proofIds
    bytes32[] public proofList;
    
    uint256 public totalProofs;
    uint256 public totalVerified;

    event ProofSubmitted(
        bytes32 indexed proofId,
        ProofType indexed proofType,
        bytes32 indexed proverId,
        bytes32 targetHash,
        uint64 epoch
    );
    event ProofVerified(bytes32 indexed proofId, bytes32 indexed verifierId);

    error ProofAlreadyExists(bytes32 proofId);
    error ProofNotFound(bytes32 proofId);
    error ProofAlreadyVerified(bytes32 proofId);

    /**
     * @notice Submit a proof
     * @param proofType Type of proof
     * @param proverId Prover node ID or DID
     * @param targetHash Target (CID, jobId, dealId)
     * @param proofHash Hash of proof data
     * @param proofCid SVDB CID for full proof
     * @param epoch Epoch number
     */
    function submitProof(
        ProofType proofType,
        bytes32 proverId,
        bytes32 targetHash,
        bytes32 proofHash,
        bytes32 proofCid,
        uint64 epoch
    ) external returns (bytes32) {
        bytes32 proofId = keccak256(abi.encodePacked(
            proofType,
            proverId,
            targetHash,
            epoch,
            block.timestamp
        ));
        
        if (proofs[proofId].timestamp != 0) revert ProofAlreadyExists(proofId);
        
        proofs[proofId] = Proof({
            proofId: proofId,
            proofType: proofType,
            proverId: proverId,
            targetHash: targetHash,
            proofHash: proofHash,
            proofCid: proofCid,
            timestamp: uint64(block.timestamp),
            epoch: epoch,
            verified: false,
            verifierId: bytes32(0)
        });
        
        proverProofs[proverId].push(proofId);
        targetProofs[targetHash].push(proofId);
        epochProofs[epoch].push(proofId);
        proofList.push(proofId);
        totalProofs++;
        
        emit ProofSubmitted(proofId, proofType, proverId, targetHash, epoch);
        
        return proofId;
    }

    /**
     * @notice Verify a proof
     * @param proofId Proof ID
     * @param verifierId Verifier node ID
     */
    function verifyProof(bytes32 proofId, bytes32 verifierId) external {
        Proof storage proof = proofs[proofId];
        
        if (proof.timestamp == 0) revert ProofNotFound(proofId);
        if (proof.verified) revert ProofAlreadyVerified(proofId);
        
        proof.verified = true;
        proof.verifierId = verifierId;
        totalVerified++;
        
        emit ProofVerified(proofId, verifierId);
    }

    /**
     * @notice Get proof details
     * @param proofId Proof ID
     * @return Proof struct
     */
    function getProof(bytes32 proofId) external view returns (Proof memory) {
        if (proofs[proofId].timestamp == 0) revert ProofNotFound(proofId);
        return proofs[proofId];
    }

    /**
     * @notice Get proofs by prover
     * @param proverId Prover ID
     * @return Array of proof IDs
     */
    function getProofsByProver(bytes32 proverId) external view returns (bytes32[] memory) {
        return proverProofs[proverId];
    }

    /**
     * @notice Get proofs by target
     * @param targetHash Target hash (CID/jobId/dealId)
     * @return Array of proof IDs
     */
    function getProofsByTarget(bytes32 targetHash) external view returns (bytes32[] memory) {
        return targetProofs[targetHash];
    }

    /**
     * @notice Get proofs by epoch
     * @param epoch Epoch number
     * @return Array of proof IDs
     */
    function getProofsByEpoch(uint64 epoch) external view returns (bytes32[] memory) {
        return epochProofs[epoch];
    }

    /**
     * @notice Check if target has valid proofs for epoch
     * @param targetHash Target hash
     * @param epoch Epoch number
     * @param minProofs Minimum number of verified proofs required
     * @return bool True if meets requirement
     */
    function hasValidProofs(
        bytes32 targetHash,
        uint64 epoch,
        uint256 minProofs
    ) external view returns (bool) {
        bytes32[] storage proofIds = targetProofs[targetHash];
        uint256 validCount = 0;
        
        for (uint256 i = 0; i < proofIds.length; i++) {
            Proof storage proof = proofs[proofIds[i]];
            if (proof.epoch == epoch && proof.verified) {
                validCount++;
                if (validCount >= minProofs) return true;
            }
        }
        
        return false;
    }

    /**
     * @notice Get proof statistics
     * @return total Total proofs
     * @return verified Verified proofs
     * @return pending Pending verification
     */
    function getProofStats() external view returns (
        uint256 total,
        uint256 verified,
        uint256 pending
    ) {
        total = totalProofs;
        verified = totalVerified;
        pending = totalProofs - totalVerified;
    }

    /**
     * @notice Get proof statistics by type
     * @param proofType Proof type to query
     * @return count Number of proofs of this type
     */
    function getProofCountByType(ProofType proofType) external view returns (uint256 count) {
        for (uint256 i = 0; i < proofList.length; i++) {
            if (proofs[proofList[i]].proofType == proofType) {
                count++;
            }
        }
    }

    /**
     * @notice Get prover reputation (verified proof rate)
     * @param proverId Prover ID
     * @return totalProofs Total proofs submitted
     * @return verifiedProofs Verified proofs
     * @return reputationScore Score 0-100
     */
    function getProverReputation(bytes32 proverId) external view returns (
        uint256 totalProofs,
        uint256 verifiedProofs,
        uint256 reputationScore
    ) {
        bytes32[] storage proofIds = proverProofs[proverId];
        totalProofs = proofIds.length;
        
        for (uint256 i = 0; i < proofIds.length; i++) {
            if (proofs[proofIds[i]].verified) {
                verifiedProofs++;
            }
        }
        
        if (totalProofs > 0) {
            reputationScore = (verifiedProofs * 100) / totalProofs;
        }
    }
}

