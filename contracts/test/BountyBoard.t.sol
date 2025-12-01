// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "forge-std/console.sol";
import {BountyBoard} from "../BountyBoard.sol";
import {ArthaCoin} from "../ArthaCoin.sol";

import "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

contract BountyBoardTest is Test {
    BountyBoard board;
    ArthaCoin token;
    address admin = address(0xA11CE);
    address validatorsPool = address(0x1111);
    address stakingRewardsPool = address(0x2222);
    address ecosystemGrantsPool = address(0x3333);
    address marketingWallet = address(0x4444);
    address developersPool = address(0x5555);
    address daoGovernancePool = address(0x6666);
    address treasuryReserve = address(0x7777);
    address ecosystemPool = address(0xEC05);
    address claimer = address(0xC1A1);

    function setUp() public {
        board = new BountyBoard();
        
        ArthaCoin implementation = new ArthaCoin();
        bytes memory initData = abi.encodeWithSelector(
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
        ERC1967Proxy proxy = new ERC1967Proxy(address(implementation), initData);
        token = ArthaCoin(address(proxy));
        // fund this contract with some ETH
        vm.deal(address(this), 10 ether);
    }

    function testCreateAndClaimEth() public {
        uint256 id = board.create{value: 1 ether}("Fix", "Do X");
        assertEq(id, 0);
        address payable me = payable(address(0xBEEF));
        uint256 beforeBal = me.balance;
        board.claim(id, me);
        assertEq(me.balance, beforeBal + 1 ether);
    }

    function testCreateAndClaimArtha() public {
        board.setArthaToken(address(token));
        // mint some ARTHA to this contract via emergencyMint for testing
        vm.prank(admin);
        token.emergencyMint(address(this), 1e18);
        // approve transfer
        // since ArthaCoin is ERC20Upgradeable, we need to cast and call approve via abi
        (bool ok1, ) = address(token).call(abi.encodeWithSignature("approve(address,uint256)", address(board), 1e18));
        require(ok1, "approve");
        uint256 id = board.createWithArtha("Fix", "Do Y", 1e18);
        address payable me = payable(address(0xABCD));
        // drain ETH so claim uses ERC20 path
        (bool ok2, ) = payable(admin).call{value: address(board).balance}("");
        ok2; // silence warning
        board.claim(id, me);
        // check ERC20 transfer
        (, bytes memory data) = address(token).call(abi.encodeWithSignature("balanceOf(address)", me));
        uint256 bal = abi.decode(data, (uint256));
        assertEq(bal, 1e18);
    }

    function testCreateAndClaim() public {
        // vm.deal(address(this), 1 ether); // Already dealt in setUp
        uint256 id = board.create{value: 0.5 ether}("Fix bug", "Details");
        // vm.deal(address(board), 0); // Reset board balance for this specific test
        vm.deal(claimer, 0); // Reset claimer balance for this specific test
        board.claim(id, payable(claimer));
        assertGt(claimer.balance, 0);
    }

    function testEcosystemFunding() public {
        vm.deal(ecosystemPool, 1 ether);
        vm.prank(ecosystemPool);
        (bool ok,) = address(board).call{value: 0.25 ether}("");
        assertTrue(ok);
        // New bounty should exist at id 0
        (address sponsor,,,uint256 reward,,) = board.bounties(0);
        console.log("Sponsor:", sponsor);
        console.log("Expected:", ecosystemPool);
        assertEq(sponsor, ecosystemPool, "sponsor mismatch");
        assertEq(reward, 0.25 ether, "reward mismatch");
    }
}
