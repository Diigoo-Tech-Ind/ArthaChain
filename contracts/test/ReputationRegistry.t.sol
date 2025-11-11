// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import {ReputationRegistry} from "../ReputationRegistry.sol";

contract ReputationRegistryTest is Test {
    ReputationRegistry internal rep;
    address admin = address(0xA11CE);
    address updater = address(0xB0B);
    address dev = address(0xD3V);

    function setUp() public {
        rep = new ReputationRegistry(admin);
        vm.prank(admin);
        rep.setUpdater(updater, true);
    }

    function testAddAndSubReputation() public {
        vm.prank(updater);
        rep.addReputation(dev, 100);
        (uint256 r,,) = rep.developers(dev);
        assertEq(r, 100);

        vm.prank(updater);
        rep.subReputation(dev, 40);
        (r,,) = rep.developers(dev);
        assertEq(r, 60);
    }

    function testContribAndSlashFree() public {
        vm.prank(updater);
        rep.recordLineageContribution(dev);
        vm.prank(updater);
        rep.recordSlashFreeEpoch(dev);
        (,uint256 lineage,uint256 epochs) = rep.developers(dev);
        assertEq(lineage, 1);
        assertEq(epochs, 1);
    }
}


