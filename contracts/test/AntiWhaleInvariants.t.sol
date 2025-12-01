// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";

import {AntiWhaleManager} from "../AntiWhaleManager.sol";

import "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

import "forge-std/StdInvariant.sol";

contract AntiWhaleInvariants is StdInvariant, Test {
    AntiWhaleManager internal anti;
    address internal admin = address(0xA11CE);
    address internal token = address(0x70C0);

    function setUp() public {
        AntiWhaleManager implementation = new AntiWhaleManager();
        bytes memory initData = abi.encodeWithSelector(
            AntiWhaleManager.initialize.selector,
            admin,
            token
        );
        ERC1967Proxy proxy = new ERC1967Proxy(address(implementation), initData);
        anti = AntiWhaleManager(address(proxy));
    }

    function testLimitsWithinBounds() public view {
        uint256 supply = 1_000_000 ether;
        uint256 maxHold = anti.getMaxHoldingAmount(supply);
        uint256 maxXfer = anti.getMaxTransferAmount(supply);
        assertEq(maxHold, (supply * 150) / 10000, "holding limit != 1.5% of supply");
        assertEq(maxXfer, (supply * 50) / 10000, "transfer limit != 0.5% of supply");
    }
}


