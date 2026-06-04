// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "./SeerArenaPoints.sol";

contract SeerPredictionRegistry {
    enum ComparisonOperator {
        Gte,
        Lte
    }

    enum Position {
        BackSeer,
        ChallengeSeer
    }

    enum Status {
        Open,
        Resolved,
        Cancelled
    }

    enum Outcome {
        Void,
        SeerCorrect,
        SeerIncorrect
    }

    struct Prediction {
        string claim;
        bytes32 dataKey;
        uint256 targetValue;
        uint256 expiryTime;
        ComparisonOperator comparisonOperator;
        Position seerPosition;
        Status status;
        Outcome outcome;
        uint256 finalValue;
    }

    struct Entry {
        Position position;
        uint256 pointsAmount;
        bool resolved;
    }

    address public owner;
    address public resolver;
    SeerArenaPoints public points;
    uint256 public nextPredictionId = 1;

    mapping(uint256 => Prediction) public predictions;
    mapping(uint256 => mapping(address => Entry)) public entries;

    event PredictionCreated(uint256 indexed predictionId, bytes32 dataKey, uint256 expiryTime);
    event PredictionEntered(uint256 indexed predictionId, address indexed user, Position position, uint256 pointsAmount);
    event PredictionResolved(uint256 indexed predictionId, Outcome outcome, uint256 finalValue);

    modifier onlyOwner() {
        require(msg.sender == owner, "NOT_OWNER");
        _;
    }

    modifier onlyResolver() {
        require(msg.sender == resolver, "NOT_RESOLVER");
        _;
    }

    constructor(address pointsAddress, address resolverAddress) {
        require(pointsAddress != address(0), "ZERO_POINTS");
        require(resolverAddress != address(0), "ZERO_RESOLVER");
        owner = msg.sender;
        points = SeerArenaPoints(pointsAddress);
        resolver = resolverAddress;
    }

    function setResolver(address nextResolver) external onlyOwner {
        require(nextResolver != address(0), "ZERO_RESOLVER");
        resolver = nextResolver;
    }

    function createPrediction(
        string calldata claim,
        bytes32 dataKey,
        uint256 targetValue,
        uint256 expiryTime,
        uint8 comparisonOperator,
        uint8 seerPosition
    ) external onlyResolver returns (uint256) {
        require(expiryTime > block.timestamp, "EXPIRY_IN_PAST");
        require(comparisonOperator <= uint8(type(ComparisonOperator).max), "BAD_OPERATOR");
        require(seerPosition <= uint8(type(Position).max), "BAD_POSITION");

        uint256 predictionId = nextPredictionId++;
        predictions[predictionId] = Prediction({
            claim: claim,
            dataKey: dataKey,
            targetValue: targetValue,
            expiryTime: expiryTime,
            comparisonOperator: ComparisonOperator(comparisonOperator),
            seerPosition: Position(seerPosition),
            status: Status.Open,
            outcome: Outcome.Void,
            finalValue: 0
        });

        emit PredictionCreated(predictionId, dataKey, expiryTime);
        return predictionId;
    }

    function enterPrediction(uint256 predictionId, uint8 position, uint256 pointsAmount) external {
        Prediction storage prediction = predictions[predictionId];
        require(prediction.expiryTime != 0, "UNKNOWN_PREDICTION");
        require(prediction.status == Status.Open, "NOT_OPEN");
        require(block.timestamp < prediction.expiryTime, "EXPIRED");
        require(position <= uint8(type(Position).max), "BAD_POSITION");
        require(pointsAmount > 0, "ZERO_POINTS");
        require(entries[predictionId][msg.sender].pointsAmount == 0, "ALREADY_ENTERED");

        points.lockPoints(msg.sender, pointsAmount);
        entries[predictionId][msg.sender] = Entry(Position(position), pointsAmount, false);

        emit PredictionEntered(predictionId, msg.sender, Position(position), pointsAmount);
    }

    function resolvePrediction(uint256 predictionId, uint8 outcome, uint256 finalValue) external onlyResolver {
        Prediction storage prediction = predictions[predictionId];
        require(prediction.expiryTime != 0, "UNKNOWN_PREDICTION");
        require(prediction.status == Status.Open, "NOT_OPEN");
        require(block.timestamp >= prediction.expiryTime, "NOT_EXPIRED");
        require(outcome <= uint8(type(Outcome).max), "BAD_OUTCOME");

        prediction.status = Status.Resolved;
        prediction.outcome = Outcome(outcome);
        prediction.finalValue = finalValue;

        emit PredictionResolved(predictionId, Outcome(outcome), finalValue);
    }
}
