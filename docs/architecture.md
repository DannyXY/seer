# Seer Backend And Contract Architecture

## Current Direction

Seer is backend-heavy, contract-light, and provider-abstracted.

```text
Rust backend handles intelligence.
Claude explains structured facts.
Nansen supplies preferred wallet/protocol intelligence.
MockProvider protects demos.
Solidity contracts prove points, predictions, identity, intents, and execution policy hashes.
```

## AI Layer

Claude is integrated through Anthropic's Messages API when `CLAUDE_API_KEY` is configured. The backend sends structured JSON facts and asks Claude for JSON-only explanations. Responses are parsed and validated before they are returned.

Claude currently supports:

- signal explanations
- intent parse explanations
- Arena prediction reasoning copy

Claude does not decide whether a signal is true, whether an intent is allowed, or whether a transaction should execute. Those decisions remain in provider data, backend rules, policy checks, and wallet/smart-account authorization.

If Claude is unavailable, Seer falls back to deterministic explanation text. This keeps the demo reliable without pretending fallback text is live AI reasoning.

## Provider Strategy

The backend should never hardcode product logic to Nansen-only fields.

Providers are represented behind `OnchainDataProvider`:

- `MockProvider`
- `NansenProvider`
- future `RpcProvider`
- future `AlliumProvider`
- future `DefiLlamaProvider`
- future `CoinGeckoProvider`

## Intent Execution

Intent execution is not wallet custody.

The system supports four trigger families:

- Instant
- Recurring
- Conditional
- Recurring conditional

Examples:

- "Buy MNT now"
- "Accumulate 25 USDC of mETH every week"
- "When mETH TVL crosses 50M, create an execution recommendation"
- "Every Monday, if portfolio risk is below 60 and mETH TVL is above 50M, execute within my policy"

The backend creates:

```text
AgentIntent
ExecutionCondition[]
ExecutionPolicy
ExecutionLog
ReasoningLog
```

Automation requires a scoped policy with:

- allowed assets
- allowed protocols
- max spend
- max transaction count
- expiry
- policy hash
- revoke path

## Authentication

Seer uses wallet-based authentication.

```text
challenge -> wallet signature -> recovered address verification -> bearer session
```

Protected routes must prove the authenticated wallet matches the wallet being acted on. This matters for:

- creating and evaluating intents
- reading wallet-specific intents
- entering Arena predictions
- reading wallet-specific Arena entries
- generating identity and SBT metadata
- relaying signed transactions
- future scoped backend-signed execution

## Contract Boundaries

`SeerArenaPoints` owns point balances.

`SeerPredictionRegistry` owns prediction lifecycle and locks points through the points contract.

`SeerIdentitySBT` mints non-transferable identity records.

`SeerIntentRegistry` anchors:

- intent hashes
- reasoning hashes
- execution policy hashes
- policy revocations

## Demo Reliability

The demo should run without live third-party dependencies.

`MockProvider` must stay realistic enough to show:

- smart-money movement
- mETH TVL thresholds
- wallet risk scoring
- identity archetypes
- Arena prediction creation
- conditional intent parsing

## Runtime Roles

The same Rust binary supports two roles:

- `APP_ROLE=api` starts the Axum API and an internal MVP scheduler.
- `APP_ROLE=worker` starts the standalone scheduler for Render Background Worker.

The worker runs interval loops for:

- signal generation and condition-trigger checks every 30 seconds
- Arena metric refresh, due resolution, and leaderboard recalculation every 5 minutes
- Arena prediction generation every 15 minutes
- wallet cohort benchmark refresh every hour

## Actionability

Seer should not stop at insight cards.

The actionable path is:

```text
provider facts
  -> condition evaluation
  -> execution proposal
  -> user signature or scoped delegated policy
  -> Mantle testnet transaction relay
  -> reasoning/intent/action hash anchoring
```

For MVP, `POST /api/contracts/send-raw-transaction` supports user-signed transaction relay through Mantle RPC. Backend-signed automation is deliberately gated behind signer configuration and execution policy checks.

When an intent is activated, Seer evaluates it immediately and records an execution log. The worker also evaluates active executable intents on the fast interval, so recurring and conditional intents do not just sit in storage.

## Smart Account Session Keys

Recurring transactions require delegated authorization. Seer supports the smart-account/session-key model:

```text
smart account owner grants session key
  -> policy limits assets/protocols/contracts/spend/count/expiry
  -> Seer evaluates active intent
  -> policy enforcement passes
  -> user-operation draft is produced
  -> AA provider/bundler submits operation
```

This avoids broad wallet custody. Seer never gets unlimited EOA control; it can only act inside the policy the user granted and can revoke.

`AA_BUNDLER_URL` enables ERC-4337 relay through:

- `eth_sendUserOperation`
- `eth_getUserOperationReceipt`

Provider-specific work remains explicit: a smart-account SDK must build/sign the user operation and apply paymaster/session-key rules before Seer relays it.

## Action Builder

Seer includes a first concrete transaction builder for ERC-20 approvals. If token and strategy addresses are configured, an accumulate intent such as `accumulate 25 USDC weekly` can produce runnable Mantle calldata:

```text
to = MANTLE_USDC_ADDRESS
data = approve(SEER_APPROVED_STRATEGY_ADDRESS, 25 USDC units)
```

This is the first step before protocol-specific swap/deposit builders. Those builders should be added per protocol with ABI, quote, slippage, allowance, and risk checks.

Seer can also call ERC-20 `allowance(owner, spender)` through Mantle RPC. If allowance already covers the intended spend amount, approval is skipped.
