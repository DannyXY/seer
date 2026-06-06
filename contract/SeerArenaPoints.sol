// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

contract SeerArenaPoints {
    uint256 public constant STARTER_POINTS = 1000;

    address public owner;
    address public arena;

    mapping(address => uint256) private availablePoints;
    mapping(address => uint256) private lockedPoints;
    mapping(address => bool) public claimedStarterPoints;

    event ArenaUpdated(address indexed arena);
    event StarterPointsClaimed(address indexed user, uint256 amount);
    event PointsLocked(address indexed user, uint256 amount);
    event PointsSettled(address indexed user, int256 pointsDelta);
    event LockedPointsSettled(address indexed user, uint256 lockedAmount, int256 pointsDelta);

    modifier onlyOwner() {
        require(msg.sender == owner, "NOT_OWNER");
        _;
    }

    modifier onlyArena() {
        require(msg.sender == arena, "NOT_ARENA");
        _;
    }

    constructor() {
        owner = msg.sender;
    }

    function setArena(address nextArena) external onlyOwner {
        require(nextArena != address(0), "ZERO_ARENA");
        arena = nextArena;
        emit ArenaUpdated(nextArena);
    }

    function claimStarterPoints() external {
        require(!claimedStarterPoints[msg.sender], "ALREADY_CLAIMED");
        claimedStarterPoints[msg.sender] = true;
        availablePoints[msg.sender] += STARTER_POINTS;
        emit StarterPointsClaimed(msg.sender, STARTER_POINTS);
    }

    function lockPoints(address user, uint256 amount) external onlyArena {
        require(availablePoints[user] >= amount, "INSUFFICIENT_POINTS");
        availablePoints[user] -= amount;
        lockedPoints[user] += amount;
        emit PointsLocked(user, amount);
    }

    function settlePoints(address user, int256 pointsDelta) external onlyArena {
        uint256 lockedAmount = lockedPoints[user];
        _settleLockedPoints(user, lockedAmount, pointsDelta);
        emit PointsSettled(user, pointsDelta);
    }

    function settleLockedPoints(address user, uint256 lockedAmount, int256 pointsDelta) external onlyArena {
        _settleLockedPoints(user, lockedAmount, pointsDelta);
        emit LockedPointsSettled(user, lockedAmount, pointsDelta);
    }

    function getTotalPoints(address user) external view returns (uint256) {
        return availablePoints[user] + lockedPoints[user];
    }

    function getAvailablePoints(address user) external view returns (uint256) {
        return availablePoints[user];
    }

    function getLockedPoints(address user) external view returns (uint256) {
        return lockedPoints[user];
    }

    function _settleLockedPoints(address user, uint256 lockedAmount, int256 pointsDelta) internal {
        require(lockedPoints[user] >= lockedAmount, "LOCKED_EXCEEDS_BALANCE");

        lockedPoints[user] -= lockedAmount;
        if (pointsDelta >= 0) {
            availablePoints[user] += lockedAmount + uint256(pointsDelta);
        } else {
            uint256 loss = uint256(-pointsDelta);
            require(lockedAmount >= loss, "LOSS_EXCEEDS_LOCKED");
            availablePoints[user] += lockedAmount - loss;
        }
    }
}
