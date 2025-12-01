// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import {AntiWhaleManager} from "../AntiWhaleManager.sol";

import "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

contract AntiWhaleExemptionsTest is Test {
    AntiWhaleManager internal anti;
    address internal admin = address(0xA11CE);

    address internal validatorsPool = address(0x1111);
    address internal rewardsPool = address(0x2222);
    address internal ecosystemGrantsPool = address(0x3333);
    address internal marketingWallet = address(0x4444);
    address internal developersPool = address(0x5555);
    address internal daoGovernancePool = address(0x6666);
    address internal treasuryReserve = address(0x7777);

    function setUp() public {
        AntiWhaleManager implementation = new AntiWhaleManager();
        bytes memory initData = abi.encodeWithSelector(
            AntiWhaleManager.initialize.selector,
            admin,
            address(0x70C0)
        );
        ERC1967Proxy proxy = new ERC1967Proxy(address(implementation), initData);
        anti = AntiWhaleManager(address(proxy));
    }

    function testSetExemptionsForSystemPools() public {
        vm.prank(admin);
        anti.setExempt(validatorsPool, true);
        vm.prank(admin);
        anti.setExempt(rewardsPool, true);
        vm.prank(admin);
        anti.setExempt(ecosystemGrantsPool, true);
        vm.prank(admin);
        anti.setExempt(marketingWallet, true);
        vm.prank(admin);
        anti.setExempt(developersPool, true);
        vm.prank(admin);
        anti.setExempt(daoGovernancePool, true);
        vm.prank(admin);
        anti.setExempt(treasuryReserve, true);

        assertTrue(anti.isExempt(validatorsPool));
        assertTrue(anti.isExempt(rewardsPool));
        assertTrue(anti.isExempt(ecosystemGrantsPool));
        assertTrue(anti.isExempt(marketingWallet));
        assertTrue(anti.isExempt(developersPool));
        assertTrue(anti.isExempt(daoGovernancePool));
        assertTrue(anti.isExempt(treasuryReserve));
    }
}


