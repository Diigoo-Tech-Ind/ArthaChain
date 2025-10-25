// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title SVDB Proofs V2 and V3
/// @notice V2: time-salted inclusion; V3: batch verification for gas efficiency
contract SVDBProofsV2V3 {
    /// @notice Verify time-salted inclusion (PoSt-lite): keccak(root||salt) equals composed branch
    function verifySalted(
        bytes32 root,
        bytes32 salt,
        bytes32 leaf,
        bytes32[] calldata branch,
        uint256 index
    ) external pure returns (bool) {
        bytes32 acc = leaf;
        uint256 idx = index;
        for (uint256 i = 0; i < branch.length; i++) {
            acc = (idx % 2 == 0) ? keccak256(abi.encodePacked(acc, branch[i])) : keccak256(abi.encodePacked(branch[i], acc));
            idx >>= 1;
        }
        bytes32 saltedRoot = keccak256(abi.encodePacked(root, salt));
        return acc == saltedRoot;
    }

    /// @notice Batch-verify multiple salted proofs; returns number of valid proofs
    function batchVerifySalted(
        bytes32[] calldata roots,
        bytes32[] calldata salts,
        bytes32[] calldata leaves,
        bytes32[][] calldata branches,
        uint256[] calldata indices
    ) external pure returns (uint256 validCount) {
        require(
            roots.length == salts.length &&
            roots.length == leaves.length &&
            roots.length == branches.length &&
            roots.length == indices.length,
            "len"
        );
        uint256 n = roots.length;
        for (uint256 i = 0; i < n; i++) {
            bytes32 acc = leaves[i];
            uint256 idx = indices[i];
            for (uint256 j = 0; j < branches[i].length; j++) {
                acc = (idx % 2 == 0) ? keccak256(abi.encodePacked(acc, branches[i][j])) : keccak256(abi.encodePacked(branches[i][j], acc));
                idx >>= 1;
            }
            if (acc == keccak256(abi.encodePacked(roots[i], salts[i]))) {
                validCount += 1;
            }
        }
    }
}


