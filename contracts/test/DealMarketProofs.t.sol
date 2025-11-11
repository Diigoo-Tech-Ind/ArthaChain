// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import {DealMarket} from "../DealMarket.sol";

contract StrictProofManager {
    function verifyMerkleSample(bytes32 root, bytes32 leaf, bytes32[] calldata branch, uint256 index) external pure returns (bool) {
        bytes32 acc = leaf; uint256 idx = index;
        for (uint256 i = 0; i < branch.length; i++) {
            bytes32 sib = branch[i];
            acc = (idx % 2 == 0) ? keccak256(abi.encodePacked(acc, sib)) : keccak256(abi.encodePacked(sib, acc));
            idx >>= 1;
        }
        return acc == root;
    }
}

contract DealMarketProofsTest is Test {
    DealMarket internal market;
    StrictProofManager internal pm;
    address provider = address(0xBEEF);

    function setUp() public {
        pm = new StrictProofManager();
        market = new DealMarket(address(pm), 1e15);
    }

    function testProofFailsWithBadLeaf() public {
        bytes32 root = keccak256(abi.encodePacked(bytes32(uint256(1)), bytes32(uint256(2))));
        uint64 size = 1 << 30; uint32 replicas = 1; uint32 months = 1;
        uint256 endowment = 1e15;
        market.createDeal{value: endowment}(root, size, replicas, months);
        bytes32 leaf = bytes32(uint256(3));
        bytes32[] memory branch = new bytes32[](1); branch[0] = bytes32(uint256(2));
        vm.prank(provider);
        vm.expectRevert();
        market.streamPayout(root, leaf, branch, 0);
    }
}


