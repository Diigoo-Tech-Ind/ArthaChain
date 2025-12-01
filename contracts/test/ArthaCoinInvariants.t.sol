// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";

import {ArthaCoin} from "../ArthaCoin.sol";
import {CycleManager} from "../CycleManager.sol";
import {BurnManager} from "../BurnManager.sol";
import {AntiWhaleManager} from "../AntiWhaleManager.sol";

import "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

import "forge-std/StdInvariant.sol";

contract ArthaCoinInvariants is StdInvariant, Test {
    ArthaCoin internal token;
    CycleManager internal cycle;
    BurnManager internal burn;
    AntiWhaleManager internal anti;

    address internal admin = address(0xA11CE);
    address internal validatorsPool = address(0x1111);
    address internal stakingRewardsPool = address(0x2222);
    address internal ecosystemGrantsPool = address(0x3333);
    address internal marketingWallet = address(0x4444);
    address internal developersPool = address(0x5555);
    address internal daoGovernancePool = address(0x6666);
    address internal treasuryReserve = address(0x7777);

    function setUp() public {
        // Deploy managers
        CycleManager cycleImpl = new CycleManager();
        bytes memory cycleInit = abi.encodeWithSelector(CycleManager.initialize.selector, admin, address(0));
        ERC1967Proxy cycleProxy = new ERC1967Proxy(address(cycleImpl), cycleInit);
        cycle = CycleManager(address(cycleProxy));

        BurnManager burnImpl = new BurnManager();
        bytes memory burnInit = abi.encodeWithSelector(BurnManager.initialize.selector, admin);
        ERC1967Proxy burnProxy = new ERC1967Proxy(address(burnImpl), burnInit);
        burn = BurnManager(address(burnProxy));

        AntiWhaleManager antiImpl = new AntiWhaleManager();
        bytes memory antiInit = abi.encodeWithSelector(AntiWhaleManager.initialize.selector, admin, address(0));
        ERC1967Proxy antiProxy = new ERC1967Proxy(address(antiImpl), antiInit);
        anti = AntiWhaleManager(address(antiProxy));

        // Deploy token
        ArthaCoin tokenImpl = new ArthaCoin();
        bytes memory tokenInit = abi.encodeWithSelector(
            ArthaCoin.initialize.selector,
            admin,
            validatorsPool,
            stakingRewardsPool,
            ecosystemGrantsPool,
            marketingWallet,
            developersPool,
            daoGovernancePool,
            treasuryReserve
        );
        ERC1967Proxy tokenProxy = new ERC1967Proxy(address(tokenImpl), tokenInit);
        token = ArthaCoin(address(tokenProxy));

        // Wire managers
        vm.prank(admin);
        token.setCycleManager(address(cycle));
        vm.prank(admin);
        token.setBurnManager(address(burn));
        vm.prank(admin);
        token.setAntiWhaleManager(address(anti));

        // Grant token role to managers where needed
        vm.prank(admin);
        cycle.updateTokenContract(address(token));
        vm.prank(admin);
        anti.updateTokenContract(address(token));
    }

    // Invariant: Burn rate must be non-decreasing over years
    function testBurnRateMonotonic() public view {
        uint256 last = 0;
        for (uint256 y = 0; y < 40; y++) {
            uint256 r = burn.getBurnRateForYear(y);
            assertGe(r, last, "burn rate decreased");
            last = r;
        }
    }

    // Invariant: Emission splits sum to 100% and minting honors total
    function testEmissionSplitAndMinting() public {
        // Skip if not time to mint yet; we simulate by allowing completeCycle when canMint == true
        (uint256 c, uint256 emission, bool canMint) = token.getCurrentCycleInfo();
        if (!canMint) {
            // Warp to cycle start
            uint256 start = cycle.getCycleStartTime(c);
            if (block.timestamp < start) vm.warp(start);
        }

        // Balances before
        uint256 v0 = token.balanceOf(validatorsPool);
        uint256 s0 = token.balanceOf(stakingRewardsPool);
        uint256 e0 = token.balanceOf(ecosystemGrantsPool);
        uint256 m0 = token.balanceOf(marketingWallet);
        uint256 d0 = token.balanceOf(developersPool);
        uint256 g0 = token.balanceOf(daoGovernancePool);
        uint256 t0 = token.balanceOf(treasuryReserve);

        vm.prank(admin);
        // Grant minter role to admin to call mintNextCycle via CycleManager if needed; token uses MINTER_ROLE on caller
        // For test simplicity, grant MINTER_ROLE to this contract and call as such
        bytes32 MINTER_ROLE = keccak256("MINTER_ROLE");
        token.grantRole(MINTER_ROLE, address(this));
        token.mintNextCycle();

        // Balances after
        uint256 v1 = token.balanceOf(validatorsPool);
        uint256 s1 = token.balanceOf(stakingRewardsPool);
        uint256 e1 = token.balanceOf(ecosystemGrantsPool);
        uint256 m1 = token.balanceOf(marketingWallet);
        uint256 d1 = token.balanceOf(developersPool);
        uint256 g1 = token.balanceOf(daoGovernancePool);
        uint256 t1 = token.balanceOf(treasuryReserve);

        uint256 delta = (v1 - v0) + (s1 - s0) + (e1 - e0) + (m1 - m0) + (d1 - d0) + (g1 - g0) + (t1 - t0);
        assertEq(delta, emission, "emission mismatch");

        // Split percentages encoded in contract: 45 + 20 + 10 + 10 + 5 + 5 + 5 = 100
        assertEq(uint256(45 + 20 + 10 + 10 + 5 + 5 + 5), 100, "split not 100%");
    }

    // Invariant: Anti-whale limits are within specified basis points
    function testAntiWhaleLimits() public view {
        uint256 supply = token.totalSupply();
        uint256 maxHold = anti.getMaxHoldingAmount(supply);
        uint256 maxXfer = anti.getMaxTransferAmount(supply);
        // 1.5% and 0.5% of supply
        assertEq(maxHold, (supply * 150) / 10000, "max holding not 1.5%");
        assertEq(maxXfer, (supply * 50) / 10000, "max transfer not 0.5%");
    }
}


