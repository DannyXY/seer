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
- `DefiLlamaProvider`
- future `RpcProvider`
- future `AlliumProvider`
- future `CoinGeckoProvider`

`ProviderRegistry` is the runtime provider facade. If Nansen is configured but a method is unavailable or fails, protocol metrics fall through to `DefiLlamaProvider` before `MockProvider`. Other methods still fall back to `MockProvider`. This keeps wallet summaries, signals, identity generation, and intent condition checks available during demo or partial provider outages.

Nansen `portfolio/defi-holdings` is wired for wallet positions and profile summaries. Nansen `smart-money/holdings` and `token-screener` are wired for Signal Engine smart-money movement inputs. Token God Mode `tgm/flows` and `tgm/holders` are wired for token-flow signals when the corresponding Mantle token address is configured. DeFiLlama protocol TVL and yield pool data are wired for TVL/APY condition checks. The remaining Nansen surface should be mapped only after its response schema is confirmed:

- Nansen-native protocol metrics for TVL, APY, and risk conditions

Signals preserve the concrete provider source from their underlying facts. Smart-money signals can report `nansen`, protocol metric signals can report `defillama`, and fallback signals report `mock`.

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

Arena settlement is per entry. `SeerPredictionRegistry.settleEntry(predictionId, user)` settles only that entry's locked points through `SeerArenaPoints.settleLockedPoints`, preserving other active locked points for the same user.

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

- `APP_ROLE=api` starts the Axum API and can start an internal MVP scheduler.
- `APP_ROLE=worker` starts the standalone scheduler for Render Background Worker.

Set `RUN_INTERNAL_JOBS=false` on the API service when a standalone worker is running. It defaults to `true` so local `cargo run` still evaluates demo jobs without a second process.

The worker runs interval loops for:

- signal generation and condition-trigger checks every hour
- Arena due-prediction resolution (DB persistence plus on-chain resolvePrediction/settleEntry) every hour
- Arena prediction generation (from live protocol metrics, registered on-chain) every 2 hours
- wallet cohort benchmark refresh every 4 hours (job currently a stub)

Each tick also fires once at startup, so a fresh deploy resolves due predictions and generates a new one without waiting a full interval.

## Persistence

PostgreSQL is optional for local demo runs but active when `DATABASE_URL` is configured. The current durable slice persists:

- created agent intents into `agent_intents`
- execution policy drafts and session-key policies into `agent_execution_policies`
- agent execution logs into `agent_execution_logs`
- intent and policy lifecycle status changes for activation, pause, cancellation, and revocation
- generated signal snapshots into `signals`
- fast worker tick summaries into `job_runs`

Delegated execution logs include the `policy_id` of the session policy that authorized the draft, preserving the audit link between intent, policy, and proposed action.

In-memory state remains the fallback so the MVP can still run when external services are unavailable.

`RUN_MIGRATIONS=true` runs SQLx migrations at startup. Keep it disabled for local mock-only demos unless a reachable Postgres instance is available.

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
  -> Safe ERC-4337 provider builds/signs the complete operation
  -> bundler submits operation
```

This avoids broad wallet custody. Seer never gets unlimited EOA control; it can only act inside the policy the user granted and can revoke.

`AA_PROVIDER_STACK` defaults to `safe-4337-relay-kit`, matching the selected Safe ERC-4337 path. Safe's ERC-4337 support uses the Safe4337Module with an EntryPoint-compatible user-operation flow, while Safe's Relay Kit exposes the provider-side flow for creating, signing, and submitting those operations. Seer keeps this boundary non-custodial: it validates policy and relays provider-built operations, but does not hold a broad owner key.

`AA_ENTRY_POINT_ADDRESS` and `AA_BUNDLER_URL` enable ERC-4337 relay through:

- `eth_sendUserOperation`
- `eth_getUserOperationReceipt`

`AA_PAYMASTER_URL` is optional and reserved for sponsored execution. Provider-specific work remains explicit: the Safe provider SDK must build/sign the user operation and apply paymaster/session-key rules before Seer relays it.

`GET /api/contracts/readiness` exposes `live_validation.safe_user_operation`, which reports the selected provider stack, missing bundler/entry-point/RPC configuration, and the next submit step for a complete Safe user operation.

## Action Builder

Seer includes concrete transaction builders for ERC-20 approvals and a configurable strategy deposit call. If token and strategy addresses are configured, an accumulate intent such as `accumulate 25 USDC weekly` can produce runnable Mantle calldata:

```text
to = MANTLE_USDC_ADDRESS
data = approve(SEER_APPROVED_STRATEGY_SPENDER_ADDRESS || SEER_APPROVED_STRATEGY_ADDRESS, 25 USDC units)
```

After allowance is sufficient, Seer can draft the next strategy call:

```text
to = SEER_APPROVED_STRATEGY_ADDRESS
data = deposit(MANTLE_USDC_ADDRESS, 25 USDC units)
```

The deposit ABI is configured with `SEER_STRATEGY_DEPOSIT_FUNCTION`, defaulting to `deposit(address,uint256)`.

If the parsed intent names a configured protocol, Seer routes the strategy call to that protocol-specific destination first:

- `SEER_MERCHANT_MOE_STRATEGY_ADDRESS`
- `SEER_LENDLE_STRATEGY_ADDRESS`
- `SEER_AGNI_STRATEGY_ADDRESS`
- `SEER_METH_STRATEGY_ADDRESS`

Each destination can override the default function signature with its matching `*_DEPOSIT_FUNCTION` variable.

Each destination can also set a distinct ERC-20 approval spender:

- `SEER_MERCHANT_MOE_SPENDER_ADDRESS`
- `SEER_LENDLE_SPENDER_ADDRESS`
- `SEER_AGNI_SPENDER_ADDRESS`
- `SEER_METH_SPENDER_ADDRESS`

If a spender override is omitted, Seer approves the destination strategy address. This matters because many DeFi actions approve a router, vault, or pool contract that is not always the same address as the final execution target.

`GET /api/contracts/execution-readiness` exposes which token and protocol destinations are configured, so demos and operators can see whether named protocol execution is genuinely available.

If an intent explicitly names Merchant Moe, Lendle, or Agni Finance, Seer requires that protocol's destination configuration. It does not silently route those intents to the generic strategy address. Asset-only mETH intents may still use the generic strategy unless `SEER_METH_STRATEGY_ADDRESS` is configured.

Protocol-specific hardening still remains explicit. Production builders should be added per protocol with ABI, quote, slippage, allowance, and risk checks.

Seer can also call ERC-20 `allowance(owner, spender)` through Mantle RPC. If allowance already covers the intended spend amount, approval is skipped and the configured strategy call can be produced.

For `evaluate-intent-with-allowance`, Seer derives the allowance token and spender from the parsed intent when the asset and destination protocol are configured. Caller-supplied token or spender values are rejected if they conflict with the configured destination. Manual token/spender values are only needed when the protocol is intentionally left unconfigured.

Concrete drafts returned by `evaluate-intent-with-allowance` are simulated with Mantle RPC `eth_call` before the response is surfaced. If simulation fails, Seer replaces the runnable draft with a `simulation_failed` draft that omits `to` and `data`, so clients do not prompt users to sign calldata that already reverted in dry-run.

`POST /api/contracts/simulate-transaction` exposes the same dry-run boundary for explicit client checks.

`GET /api/contracts/readiness` also exposes `live_validation.protocol_swaps`, which reports runnable protocols honestly: a protocol counts as runnable only when its strategy and spender are configured and its deposit function is one the transaction builder can encode (`deposit(address,uint256)`, `deposit(address,uint256,address,uint16)`, or `supply(address,uint256,address,uint16)`). Protocols configured with swap-style signatures are listed under `configured_not_runnable` with the reason.

Execution proposals include `allowance_check` when Seer can derive the ERC-20 allowance target. This makes approval routing auditable before a client signs or relays anything:

```text
allowance_check.token_address
allowance_check.owner_address
allowance_check.spender_address
```

Condition evaluations include provider provenance:

```text
condition.observed_value
condition.source_provider
condition.source_captured_at
```

This keeps triggered actions explainable: Seer can show which TVL, APY, or risk fact caused an instant, recurring, conditional, or recurring-conditional intent to become actionable.

### Verified Protocol Adapter Notes

Lendle publishes Mantle LendingPool addresses and follows an Aave-style approval then deposit/supply flow. Seer supports configured Lendle calldata for:

```text
deposit(address,uint256,address,uint16)
supply(address,uint256,address,uint16)
```

The `onBehalfOf` argument is the intent wallet or smart account, and `referralCode` is encoded as `0`.

Merchant Moe publishes Mantle router addresses, including `MoeRouter`, `LFJ Aggregator Router`, `LB Router`, and `LB Quoter`, but Seer should not hardcode Merchant Moe execution until the specific action path is selected and verified:

- swap vs Liquidity Book liquidity add
- router ABI and function signature
- pair/path/bin parameters
- quote source and slippage bound
- exact spender address

Until then, Merchant Moe remains a named configurable destination rather than a hardcoded adapter.
