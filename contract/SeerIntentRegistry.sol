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

    /// Track all intent IDs belonging to a wallet for easy on-chain recovery.
    mapping(address => uint256[]) public userIntentIds;

    // ── Events ────────────────────────────────────────────────────────────────

    /// metadataURI is included so backends can reconstruct intent data from
    /// logs alone without a separate eth_call.
    event IntentRegistered(
        uint256 indexed intentId,
        address indexed user,
        bytes32 intentHash,
        string metadataURI
    );
    event IntentStatusUpdated(uint256 indexed intentId, IntentStatus status);
    event ReasoningLogAdded(uint256 indexed intentId, bytes32 reasoningHash, string metadataURI);
    event ExecutionPolicyRegistered(uint256 indexed policyId, uint256 indexed intentId, bytes32 policyHash, uint256 expiresAt);
    event ExecutionPolicyRevoked(uint256 indexed policyId);

    // ── Modifiers ─────────────────────────────────────────────────────────────

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

    // ── Intent lifecycle ──────────────────────────────────────────────────────

    function registerIntent(bytes32 intentHash, string calldata metadataURI) external returns (uint256) {
        uint256 intentId = nextIntentId++;
        intents[intentId] = Intent(msg.sender, intentHash, metadataURI, IntentStatus.Draft);
        userIntentIds[msg.sender].push(intentId);
        emit IntentRegistered(intentId, msg.sender, intentHash, metadataURI);
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

    /// Convenience: pause an active intent.
    function pauseIntent(uint256 intentId) external {
        Intent storage intent = intents[intentId];
        require(intent.user != address(0), "UNKNOWN_INTENT");
        require(msg.sender == intent.user || msg.sender == backendSigner, "NOT_AUTHORIZED");
        require(intent.status == IntentStatus.Active, "NOT_ACTIVE");
        intent.status = IntentStatus.Paused;
        emit IntentStatusUpdated(intentId, IntentStatus.Paused);
    }

    /// Convenience: resume a paused intent.
    function resumeIntent(uint256 intentId) external {
        Intent storage intent = intents[intentId];
        require(intent.user != address(0), "UNKNOWN_INTENT");
        require(msg.sender == intent.user || msg.sender == backendSigner, "NOT_AUTHORIZED");
        require(intent.status == IntentStatus.Paused, "NOT_PAUSED");
        intent.status = IntentStatus.Active;
        emit IntentStatusUpdated(intentId, IntentStatus.Active);
    }

    /// Convenience: cancel an intent (irreversible).
    function cancelIntent(uint256 intentId) external {
        Intent storage intent = intents[intentId];
        require(intent.user != address(0), "UNKNOWN_INTENT");
        require(msg.sender == intent.user || msg.sender == backendSigner, "NOT_AUTHORIZED");
        require(
            intent.status != IntentStatus.Cancelled && intent.status != IntentStatus.Completed,
            "ALREADY_TERMINAL"
        );
        intent.status = IntentStatus.Cancelled;
        emit IntentStatusUpdated(intentId, IntentStatus.Cancelled);
    }

    // ── View helpers ──────────────────────────────────────────────────────────

    /// Return all intent IDs registered by a wallet.
    /// Backends can call this then read intents[id] for each, or replay
    /// IntentRegistered logs — both paths give full intent data.
    function getUserIntentIds(address user) external view returns (uint256[] memory) {
        return userIntentIds[user];
    }

    // ── Reasoning & policies ──────────────────────────────────────────────────

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
