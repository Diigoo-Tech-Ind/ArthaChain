// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {ArthaCoin} from "../ArthaCoin.sol";
import {CycleManager} from "../CycleManager.sol";
import {BurnManager} from "../BurnManager.sol";
import {AntiWhaleManager} from "../AntiWhaleManager.sol";

contract ArthaCoinEchidna {
    ArthaCoin internal token;
    CycleManager internal cycle;
    BurnManager internal burn;
    AntiWhaleManager internal anti;

    address internal admin = address(0xA11CE);

    constructor() {
        cycle = new CycleManager();
        cycle.initialize(admin, address(0));
        burn = new BurnManager();
        burn.initialize(admin);
        anti = new AntiWhaleManager();
        anti.initialize(admin, address(0));

        token = new ArthaCoin();
        token.initialize(
            admin,
            address(0x1111),
            address(0x2222),
            address(0x3333),
            address(0x4444),
            address(0x5555),
            address(0x6666),
            address(0x7777)
        );

        token.setCycleManager(address(cycle));
        token.setBurnManager(address(burn));
        token.setAntiWhaleManager(address(anti));

        cycle.updateTokenContract(address(token));
        anti.updateTokenContract(address(token));
    }

    // Property: burn schedule monotonic
    function echidna_burn_rate_monotonic() public view returns (bool) {
        uint256 last = 0;
        for (uint256 y = 0; y < 40; y++) {
            uint256 r = burn.getBurnRateForYear(y);
            if (r < last) return false;
            last = r;
        }
        return true;
    }
}


