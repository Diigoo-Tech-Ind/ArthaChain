// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import {UpgradeCouncilTimelock} from "../UpgradeCouncilTimelock.sol";

contract Target {
    uint256 public x;
    function setX(uint256 v) external { x = v; }
}

contract UpgradeCouncilTimelockTest is Test {
    UpgradeCouncilTimelock council;
    Target target;
    address s1 = address(0x1);
    address s2 = address(0x2);
    address s3 = address(0x3);

    function setUp() public {
        address[] memory signers = new address[](3);
        signers[0] = s1; signers[1] = s2; signers[2] = s3;
        council = new UpgradeCouncilTimelock(signers, 2);
        target = new Target();
    }

    function testProposeApproveExecute() public {
        bytes memory data = abi.encodeWithSignature("setX(uint256)", 42);
        vm.prank(s1);
        bytes32 id = council.propose(address(target), 0, data);
        vm.prank(s2);
        council.approve(id);
        // warp past delay
        vm.warp(block.timestamp + 48 hours + 1);
        council.execute(id);
        assertEq(target.x(), 42);
    }
}


