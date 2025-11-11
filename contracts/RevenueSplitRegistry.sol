// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title Revenue Split Registry
/// @notice Records revenue split agreements for models/datasets
contract RevenueSplitRegistry {
    struct Split {
        address owner;
        address[] payees;
        uint256[] bps; // basis points, sum to 10000
        bool exists;
    }

    mapping(bytes32 => Split) private splits; // key: modelCidRoot

    event SplitSet(bytes32 indexed key, address indexed owner);

    function setSplit(bytes32 key, address[] calldata payees, uint256[] calldata bps) external {
        require(payees.length == bps.length, "len");
        uint256 total;
        for (uint256 i = 0; i < bps.length; i++) total += bps[i];
        require(total == 10000, "sum");
        Split storage s = splits[key];
        if (s.exists) {
            require(s.owner == msg.sender, "owner");
        } else {
            s.owner = msg.sender;
        }
        delete s.payees;
        delete s.bps;
        for (uint256 i = 0; i < payees.length; i++) { s.payees.push(payees[i]); s.bps.push(bps[i]); }
        s.exists = true;
        emit SplitSet(key, msg.sender);
    }

    function getSplit(bytes32 key) external view returns (address owner, address[] memory payees, uint256[] memory bps, bool exists) {
        Split storage s = splits[key];
        return (s.owner, s.payees, s.bps, s.exists);
    }
}


