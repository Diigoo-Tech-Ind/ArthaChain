// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import {DeveloperReputation} from "../DeveloperReputation.sol";

contract DeveloperReputationTest is Test {
    DeveloperReputation rep;
    address dev = address(0xD3V);

    function setUp() public {
        rep = new DeveloperReputation();
    }

    function testScoreIncreasesWithContributions() public {
        rep.recordTaskCompleted(dev);
        rep.recordModelPublished(dev);
        rep.recordSlashFreeEpoch(dev);
        uint256 score = rep.reputationScore(dev);
        assertGt(score, 0);
    }

    function testPenaltyForSlash() public {
        rep.recordTaskCompleted(dev);
        rep.recordTaskCompleted(dev);
        uint256 beforeScore = rep.reputationScore(dev);
        rep.recordSlash(dev);
        uint256 afterScore = rep.reputationScore(dev);
        assertLt(afterScore, beforeScore);
    }
}


