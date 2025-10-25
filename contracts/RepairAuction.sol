// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title RepairAuction
/// @notice Bounties for missing shards; any SP can repair and claim
contract RepairAuction {
    struct RepairTask {
        address client;
        bytes32 manifestRoot; // dataset root to repair
        uint256 shardIndex;   // missing shard index
        uint256 bountyWei;    // bounty for successful repair
        uint64 createdAt;
        bool open;
        address winner;
    }

    mapping(bytes32 => RepairTask) public tasks; // key = keccak(manifestRoot, shardIndex)

    event TaskCreated(bytes32 indexed key, bytes32 indexed manifestRoot, uint256 shardIndex, uint256 bountyWei);
    event TaskClaimed(bytes32 indexed key, address indexed provider);
    event BountyPaid(bytes32 indexed key, address indexed provider, uint256 amount);

    function _taskKey(bytes32 manifestRoot, uint256 shardIndex) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked(manifestRoot, shardIndex));
    }

    function createTask(bytes32 manifestRoot, uint256 shardIndex) external payable {
        require(msg.value > 0, "bounty");
        bytes32 key = _taskKey(manifestRoot, shardIndex);
        RepairTask storage t = tasks[key];
        require(!t.open, "exists");
        t.client = msg.sender;
        t.manifestRoot = manifestRoot;
        t.shardIndex = shardIndex;
        t.bountyWei = msg.value;
        t.createdAt = uint64(block.timestamp);
        t.open = true;
        emit TaskCreated(key, manifestRoot, shardIndex, msg.value);
    }

    /// @notice Provider claims by submitting the repaired shard hash and Merkle branch to manifest root
    function claim(bytes32 manifestRoot, uint256 shardIndex, bytes32 leaf, bytes32[] calldata branch, uint256 index) external {
        bytes32 key = _taskKey(manifestRoot, shardIndex);
        RepairTask storage t = tasks[key];
        require(t.open, "closed");

        // Verify inclusion proof matches manifestRoot
        bytes32 acc = leaf;
        uint256 idx = index;
        for (uint256 i = 0; i < branch.length; i++) {
            acc = (idx % 2 == 0) ? keccak256(abi.encodePacked(acc, branch[i])) : keccak256(abi.encodePacked(branch[i], acc));
            idx >>= 1;
        }
        require(acc == manifestRoot, "invalid");

        t.open = false;
        t.winner = msg.sender;
        uint256 amount = t.bountyWei;
        t.bountyWei = 0;
        (bool ok, ) = msg.sender.call{value: amount}("");
        require(ok, "pay");
        emit TaskClaimed(key, msg.sender);
        emit BountyPaid(key, msg.sender, amount);
    }
}


