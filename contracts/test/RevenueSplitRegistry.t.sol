// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import {RevenueSplitRegistry} from "../RevenueSplitRegistry.sol";

contract RevenueSplitRegistryTest is Test {
    RevenueSplitRegistry reg;
    bytes32 key = keccak256("modelCidRoot");

    function setUp() public {
        reg = new RevenueSplitRegistry();
    }

    function testSetAndGetSplit() public {
        address[] memory payees = new address[](2);
        payees[0] = address(0x1);
        payees[1] = address(0x2);
        uint256[] memory bps = new uint256[](2);
        bps[0] = 8000; // 80%
        bps[1] = 2000; // 20%
        reg.setSplit(key, payees, bps);
        (address owner, address[] memory p, uint256[] memory s, bool exists) = reg.getSplit(key);
        assertEq(owner, address(this));
        assertTrue(exists);
        assertEq(p.length, 2);
        assertEq(s[0], 8000);
        assertEq(s[1], 2000);
    }
}


