// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";

import {DealMarket} from "../DealMarket.sol";

contract MockProofManager {
    function verifyMerkleSample(bytes32, bytes32, bytes32[] calldata, uint256) external pure returns (bool) {
        return true;
    }
}

contract MockProofsV2V3 {
    function verifySalted(bytes32, bytes32, bytes32, bytes32[] calldata, uint256) external pure returns (bool) {
        return true;
    }
}

contract DealMarketTest is Test {
    DealMarket internal market;
    MockProofManager internal pm;
    MockProofsV2V3 internal pm2;

    address provider = address(0xBEEF);
    bytes32 root = keccak256("manifest");

    function setUp() public {
        pm = new MockProofManager();
        market = new DealMarket(address(pm), 1e15); // base price
        pm2 = new MockProofsV2V3();
        market.setProofsV2(address(pm2));
    }

    function testCreateDealAndStreamPayout() public {
        // Create a deal with endowment
        uint64 size = 1 << 30; // 1GB
        uint32 replicas = 1;
        uint32 months = 1;
        uint256 expectedEndowment = ((uint256(size) + (1<<30)-1) >> 30) * 1e15 * months * replicas;
        vm.deal(address(this), expectedEndowment);

        market.createDeal{value: expectedEndowment}(root, size, replicas, months);

        // Fund contract with some ether to pay out (endowment already sent)
        // Provide a dummy proof and trigger payout
        bytes32 leaf = keccak256("leaf");
        bytes32[] memory branch = new bytes32[](0);
        uint256 index = 0;

        vm.warp(block.timestamp + 100);
        vm.prank(provider);
        market.streamPayout(root, leaf, branch, index);
    }

    function testStreamPayoutV2() public {
        uint64 size = 1 << 30; // 1GB
        uint32 replicas = 1;
        uint32 months = 1;
        uint256 expectedEndowment = ((uint256(size) + (1<<30)-1) >> 30) * 1e15 * months * replicas;
        vm.deal(address(this), expectedEndowment);
        market.createDeal{value: expectedEndowment}(root, size, replicas, months);

        bytes32 salt = keccak256("salt");
        bytes32 leaf = keccak256("leaf");
        bytes32[] memory branch = new bytes32[](0);
        uint256 index = 0;
        
        vm.warp(block.timestamp + 100);
        vm.prank(provider);
        market.streamPayoutV2(root, salt, leaf, branch, index);
    }
}
