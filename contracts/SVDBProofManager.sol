// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title SVDB Proof Manager (Merkle Sample v1)
/// @notice Verifies Merkle inclusion proofs for SVDB manifests using blake3 leaves with keccak node composition
contract SVDBProofManager {
    /// @notice Verify a Merkle inclusion proof for a given leaf/index against a root
    /// @param root The Merkle root (32 bytes)
    /// @param leaf The leaf hash (32 bytes)
    /// @param branch The array of sibling hashes from leaf level to the root level
    /// @param index The index of the leaf in the leaves array used to compute the root
    /// @return valid True if the proof is valid
    function verifyMerkleSample(
        bytes32 root,
        bytes32 leaf,
        bytes32[] calldata branch,
        uint256 index
    ) external pure returns (bool valid) {
        bytes32 acc = leaf;
        uint256 idx = index;
        for (uint256 i = 0; i < branch.length; i++) {
            bytes32 sib = branch[i];
            if (idx % 2 == 0) {
                acc = keccak256(abi.encodePacked(acc, sib));
            } else {
                acc = keccak256(abi.encodePacked(sib, acc));
            }
            idx >>= 1;
        }
        // On-chain we use keccak256 composition over 32-byte nodes; leaves come from blake3(chunk)
        return acc == root;
    }
}


