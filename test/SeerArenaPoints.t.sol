// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "../contracts/SeerArenaPoints.sol";

contract SeerArenaPointsTest {
    SeerArenaPoints points;

    function setUp() public {
        points = new SeerArenaPoints();
        points.setArena(address(this));
    }

    function testClaimStarterPoints() public {
        points.claimStarterPoints();
        require(points.getTotalPoints(address(this)) == 1000, "starter points mismatch");
    }

    function testLockAndSettlePoints() public {
        points.claimStarterPoints();
        points.lockPoints(address(this), 100);
        require(points.getAvailablePoints(address(this)) == 900, "available mismatch");
        points.settlePoints(address(this), 50);
        require(points.getTotalPoints(address(this)) == 1050, "settled mismatch");
    }
}
