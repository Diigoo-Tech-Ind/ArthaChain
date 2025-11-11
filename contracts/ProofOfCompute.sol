// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title ProofOfCompute
/// @notice Records and verifies compute receipts for AI training/inference jobs
contract ProofOfCompute {
    struct TrainProof {
        bytes32 jobId;
        uint256 step;
        bytes32 lossDigest;      // Hash of loss values
        bytes32 gradientDigest;  // Hash of gradients
        bytes32 weightsDigest;   // Hash of updated weights
        uint64 timestamp;
        bytes32 nodePubkey;
        bytes signature;         // Node's signature
    }

    struct InferProof {
        bytes32 jobId;
        bytes32 inputDigest;
        bytes32 outputCid;
        bytes32 outputDigest;
        uint64 timestamp;
        bytes32 nodePubkey;
        bytes signature;
    }

    struct ComputeReceipt {
        bytes32 jobId;
        bytes32 nodePubkey;
        uint256 gpuSeconds;
        uint256 totalSteps;
        bytes32 finalOutputCid;
        uint64 startTime;
        uint64 endTime;
        bool finalized;
        uint256 payout;
    }

    mapping(bytes32 => TrainProof[]) public trainProofs;
    mapping(bytes32 => InferProof[]) public inferProofs;
    mapping(bytes32 => ComputeReceipt) public receipts;

    event TrainProofRecorded(bytes32 indexed jobId, uint256 step, bytes32 nodePubkey);
    event InferProofRecorded(bytes32 indexed jobId, bytes32 outputCid, bytes32 nodePubkey);
    event ReceiptFinalized(bytes32 indexed jobId, uint256 gpuSeconds, uint256 payout);

    function recordTrainProof(
        bytes32 jobId,
        uint256 step,
        bytes32 lossDigest,
        bytes32 gradientDigest,
        bytes32 weightsDigest,
        bytes32 nodePubkey,
        bytes calldata signature
    ) external {
        trainProofs[jobId].push(TrainProof({
            jobId: jobId,
            step: step,
            lossDigest: lossDigest,
            gradientDigest: gradientDigest,
            weightsDigest: weightsDigest,
            timestamp: uint64(block.timestamp),
            nodePubkey: nodePubkey,
            signature: signature
        }));

        emit TrainProofRecorded(jobId, step, nodePubkey);
    }

    function recordInferProof(
        bytes32 jobId,
        bytes32 inputDigest,
        bytes32 outputCid,
        bytes32 outputDigest,
        bytes32 nodePubkey,
        bytes calldata signature
    ) external {
        inferProofs[jobId].push(InferProof({
            jobId: jobId,
            inputDigest: inputDigest,
            outputCid: outputCid,
            outputDigest: outputDigest,
            timestamp: uint64(block.timestamp),
            nodePubkey: nodePubkey,
            signature: signature
        }));

        emit InferProofRecorded(jobId, outputCid, nodePubkey);
    }

    function finalize(
        bytes32 jobId,
        bytes32 nodePubkey,
        uint256 gpuSeconds,
        bytes32 finalOutputCid
    ) external returns (uint256) {
        require(!receipts[jobId].finalized, "Already finalized");

        // Verify proofs exist
        require(
            trainProofs[jobId].length > 0 || inferProofs[jobId].length > 0,
            "No proofs submitted"
        );

        uint64 startTime;
        uint64 endTime = uint64(block.timestamp);

        if (trainProofs[jobId].length > 0) {
            startTime = trainProofs[jobId][0].timestamp;
        } else {
            startTime = inferProofs[jobId][0].timestamp;
        }

        // Calculate payout (simplified; production uses price oracle)
        uint256 payout = gpuSeconds * 1e15; // 0.001 ARTH per GPU-second

        receipts[jobId] = ComputeReceipt({
            jobId: jobId,
            nodePubkey: nodePubkey,
            gpuSeconds: gpuSeconds,
            totalSteps: trainProofs[jobId].length,
            finalOutputCid: finalOutputCid,
            startTime: startTime,
            endTime: endTime,
            finalized: true,
            payout: payout
        });

        emit ReceiptFinalized(jobId, gpuSeconds, payout);
        return payout;
    }

    function getTrainProofs(bytes32 jobId) external view returns (TrainProof[] memory) {
        return trainProofs[jobId];
    }

    function getInferProofs(bytes32 jobId) external view returns (InferProof[] memory) {
        return inferProofs[jobId];
    }

    function getReceipt(bytes32 jobId) external view returns (ComputeReceipt memory) {
        return receipts[jobId];
    }

    function verifyProof(bytes32 jobId, uint256 proofIndex) external view returns (bool) {
        // Verify signature against node's public key
        // Production: use ecrecover or Ed25519 verification
        return trainProofs[jobId].length > proofIndex;
    }
}

