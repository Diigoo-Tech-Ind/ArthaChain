// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import {ArthaCoin} from "../ArthaCoin.sol";

contract ArthaCoinExemptionsTest is Test {
    ArthaCoin internal token;

    address internal admin = address(0xA11CE);
    address internal validatorsPool = address(0x1111);
    address internal stakingRewardsPool = address(0x2222);
    address internal ecosystemGrantsPool = address(0x3333);
    address internal marketingWallet = address(0x4444);
    address internal developersPool = address(0x5555);
    address internal daoGovernancePool = address(0x6666);
    address internal treasuryReserve = address(0x7777);

    function setUp() public {
        token = new ArthaCoin();
        token.initialize(
            admin,
            validatorsPool,
            stakingRewardsPool,
            ecosystemGrantsPool,
            marketingWallet,
            developersPool,
            daoGovernancePool,
            treasuryReserve
        );
    }

    function testPoolsAreBurnExempt() public {
        assertEq(token.burnExempt(validatorsPool), true, "validators burnExempt");
        assertEq(token.burnExempt(stakingRewardsPool), true, "rewards burnExempt");
        assertEq(token.burnExempt(ecosystemGrantsPool), true, "ecosystem burnExempt");
        assertEq(token.burnExempt(marketingWallet), true, "marketing burnExempt");
        assertEq(token.burnExempt(developersPool), true, "devs burnExempt");
        assertEq(token.burnExempt(daoGovernancePool), true, "dao burnExempt");
        assertEq(token.burnExempt(treasuryReserve), true, "treasury burnExempt");
    }
}


