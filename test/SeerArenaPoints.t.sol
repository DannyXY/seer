// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "../contract/SeerArenaPoints.sol";
import "../contract/SeerPredictionRegistry.sol";

interface Vm {
    function warp(uint256) external;
}

contract SeerArenaPointsTest {
    Vm constant vm = Vm(address(uint160(uint256(keccak256("hevm cheat code")))));
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

    function testPartialLockedSettlementPreservesOtherEntries() public {
        points.claimStarterPoints();
        points.lockPoints(address(this), 100);
        points.lockPoints(address(this), 200);
        require(points.getLockedPoints(address(this)) == 300, "locked before mismatch");

        points.settleLockedPoints(address(this), 100, 100);

        require(points.getAvailablePoints(address(this)) == 900, "available after mismatch");
        require(points.getLockedPoints(address(this)) == 200, "locked after mismatch");
        require(points.getTotalPoints(address(this)) == 1100, "total after mismatch");
    }

    function testPredictionEntrySettlementPaysWinner() public {
        points.claimStarterPoints();
        SeerPredictionRegistry registry = new SeerPredictionRegistry(address(points), address(this));
        points.setArena(address(registry));

        uint256 predictionId = registry.createPrediction(
            "mETH TVL reaches target",
            bytes32("meth_tvl"),
            50_000_000,
            block.timestamp + 1,
            0,
            0
        );

        registry.enterPrediction(predictionId, 0, 100);
        require(points.getAvailablePoints(address(this)) == 900, "available after entry mismatch");
        require(points.getLockedPoints(address(this)) == 100, "locked after entry mismatch");

        vm.warp(block.timestamp + 2);
        registry.resolvePrediction(predictionId, 1, 51_000_000);
        registry.settleEntry(predictionId, address(this));

        require(points.getAvailablePoints(address(this)) == 1100, "winner available mismatch");
        require(points.getLockedPoints(address(this)) == 0, "winner locked mismatch");
    }
}
