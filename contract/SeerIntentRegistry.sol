// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

contract SeerIntentRegistry {
    enum IntentStatus {
        Draft,
        Active,
        Paused,
        Completed,
        Cancelled
    }

    struct Intent {
        address user;
        bytes32 intentHash;
        string metadataURI;
        IntentStatus status;
    }

    struct ExecutionPolicy {
        uint256 intentId;
        bytes32 policyHash;
        uint256 expiresAt;
        string metadataURI;
        bool revoked;
    }

    address public owner;
    address public backendSigner;
    uint256 public nextIntentId = 1;
    uint256 public nextPolicyId = 1;

    mapping(uint256 => Intent) public intents;
    mapping(uint256 => ExecutionPolicy) public executionPolicies;

    event IntentRegistered(uint256 indexed intentId, address indexed user, bytes32 intentHash);
    event IntentStatusUpdated(uint256 indexed intentId, IntentStatus status);
    event ReasoningLogAdded(uint256 indexed intentId, bytes32 reasoningHash, string metadataURI);
    event ExecutionPolicyRegistered(uint256 indexed policyId, uint256 indexed intentId, bytes32 policyHash, uint256 expiresAt);
    event ExecutionPolicyRevoked(uint256 indexed policyId);

    modifier onlyOwner() {
        require(msg.sender == owner, "NOT_OWNER");
        _;
    }

    modifier onlyBackendSigner() {
        require(msg.sender == backendSigner, "NOT_BACKEND_SIGNER");
        _;
    }

    constructor(address signer) {
        require(signer != address(0), "ZERO_SIGNER");
        owner = msg.sender;
        backendSigner = signer;
    }

    function registerIntent(bytes32 intentHash, string calldata metadataURI) external returns (uint256) {
        uint256 intentId = nextIntentId++;
        intents[intentId] = Intent(msg.sender, intentHash, metadataURI, IntentStatus.Draft);
        emit IntentRegistered(intentId, msg.sender, intentHash);
        return intentId;
    }

    function updateIntentStatus(uint256 intentId, uint8 status) external {
        Intent storage intent = intents[intentId];
        require(intent.user != address(0), "UNKNOWN_INTENT");
        require(msg.sender == intent.user || msg.sender == backendSigner, "NOT_AUTHORIZED");
        require(status <= uint8(type(IntentStatus).max), "BAD_STATUS");
        intent.status = IntentStatus(status);
        emit IntentStatusUpdated(intentId, IntentStatus(status));
    }

    function addReasoningLog(uint256 intentId, bytes32 reasoningHash, string calldata metadataURI) external onlyBackendSigner {
        require(intents[intentId].user != address(0), "UNKNOWN_INTENT");
        emit ReasoningLogAdded(intentId, reasoningHash, metadataURI);
    }

    function registerExecutionPolicy(
        uint256 intentId,
        bytes32 policyHash,
        uint256 expiresAt,
        string calldata metadataURI
    ) external returns (uint256) {
        Intent storage intent = intents[intentId];
        require(intent.user == msg.sender, "NOT_INTENT_OWNER");
        require(expiresAt > block.timestamp, "EXPIRED_POLICY");

        uint256 policyId = nextPolicyId++;
        executionPolicies[policyId] = ExecutionPolicy(intentId, policyHash, expiresAt, metadataURI, false);
        emit ExecutionPolicyRegistered(policyId, intentId, policyHash, expiresAt);
        return policyId;
    }

    function revokeExecutionPolicy(uint256 policyId) external {
        ExecutionPolicy storage policy = executionPolicies[policyId];
        Intent storage intent = intents[policy.intentId];
        require(intent.user == msg.sender || msg.sender == backendSigner, "NOT_AUTHORIZED");
        policy.revoked = true;
        emit ExecutionPolicyRevoked(policyId);
    }
}
