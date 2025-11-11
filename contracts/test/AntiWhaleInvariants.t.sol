// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "forge-std/StdInvariant.sol";

import {AntiWhaleManager} from "../AntiWhaleManager.sol";

contract AntiWhaleInvariants is Test, StdInvariant {
    AntiWhaleManager internal anti;
    address internal admin = address(0xA11CE);
    address internal token = address(0xT0C0);

    function setUp() public {
        anti = new AntiWhaleManager();
        anti.initialize(admin, token);
        targetContract(address(anti));
    }

    function invariant_LimitsWithinBounds() public view {
        uint256 supply = 1_000_000 ether;
        uint256 maxHold = anti.getMaxHoldingAmount(supply);
        uint256 maxXfer = anti.getMaxTransferAmount(supply);
        assertEq(maxHold, (supply * 150) / 10000, "holding limit != 1.5% of supply");
        assertEq(maxXfer, (supply * 50) / 10000, "transfer limit != 0.5% of supply");
    }
}


