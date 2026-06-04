# Seer

Seer is a backend-first intelligence and prediction system for the Mantle Turing Test Hackathon.

The current implementation intentionally excludes frontend work and focuses on the backend, data intelligence, agent intent model, Arena points competition, and contract anchoring.

## Architecture

```text
Rust Axum backend
  -> provider abstraction: Nansen first, mock fallback
  -> signal engine
  -> Claude explanation boundary
  -> Postgres durable state
  -> Redis/cache/job-ready state
  -> Solidity contracts on Mantle
```

## MVP Surfaces

- Wallet intelligence
- Provider abstraction
- Nansen-primary provider layer with per-method mock fallback
- Signal generation
- Claude Messages API reasoning wrapper with deterministic fallback
- Portfolio identity/SBT metadata
- Arena predictions with committed points
- Agent intents
- Instant, recurring, conditional, and recurring-conditional triggers
- Scoped execution policies for delegated automation
- Intent, reasoning, and policy hash anchoring

## Important Execution Rule

Seer does not require broad wallet custody.

Agent execution is modeled as:

```text
intent -> parsed trigger -> execution policy -> user approval or scoped delegated permission -> execution log
```

Supported trigger modes:

- `Instant`
- `Recurring`
- `Conditional`
- `RecurringConditional`

Example:

```text
When mETH TVL climbs above $50M and portfolio risk is below 60,
commit up to 25 USDC weekly into the approved mETH strategy.
```

The parser accepts common trigger variants such as `climbs to`, `reaches`, `crosses`, `exceeds`, `at least`, `below`, `under`, `at most`, `recurring`, and `recurrent`.

This becomes a policy with allowed assets, allowed protocols, spend caps, expiry, and a policy hash that can be registered on-chain.

## AI Reasoning

Claude is wired as an explanation and classification layer, not as the source of financial facts.

```text
provider/RPC facts -> backend rules and scoring -> Claude JSON explanation -> backend validation -> response
```

Set `CLAUDE_API_KEY` to use the live Anthropic Messages API. If the key is absent or Claude fails, Seer returns deterministic fallback copy so signals, intents, and demos keep working.

## Local Run

```bash
cp .env.example .env
cargo run
```

The repo uses the current stable Rust toolchain because several 2026 crate releases require Cargo support for Edition 2024 manifests even though Seer itself is written with Rust 2021 edition.

Run the standalone worker locally with:

```bash
APP_ROLE=worker cargo run
```

The API listens on `PORT`, defaulting to `10000`.

## Useful Endpoints

```http
GET  /api/health
GET  /api/version

POST /api/auth/challenge
POST /api/auth/verify

GET  /api/wallet/:address/summary
GET  /api/wallet/:address/activity
GET  /api/wallet/:address/risk

GET  /api/signals
GET  /api/signals/:id

GET  /api/identity/:address
POST /api/identity/:address/generate
POST /api/identity/:address/mint-metadata

GET  /api/arena/predictions
POST /api/arena/predictions/:id/enter
GET  /api/arena/leaderboard
POST /api/arena/resolve-due

POST /api/agent/parse-intent
POST /api/agent/evaluate-intent
POST /api/agent/evaluate-intent-with-allowance
POST /api/agent/create-intent
GET  /api/agent/:address/intents
GET  /api/agent/intent/:intent_id/reasoning
POST /api/agent/intent/:intent_id/activate
POST /api/agent/intent/:intent_id/session-policy
POST /api/agent/intent/:intent_id/delegated-execute
POST /api/agent/policy/:policy_id/revoke
POST /api/agent/intent/:intent_id/pause
POST /api/agent/intent/:intent_id/stop

GET  /api/contracts/readiness
POST /api/contracts/send-raw-transaction
POST /api/contracts/send-user-operation
POST /api/contracts/user-operation-receipt
POST /api/contracts/erc20-allowance
```

## Live Action Path

Seer should make intelligence actionable on Mantle testnet without taking broad wallet custody.

```text
intent -> condition evaluation -> execution proposal -> user signs transaction -> Seer relays signed tx to Mantle RPC
```

`POST /api/agent/evaluate-intent` evaluates parsed conditions against provider facts and returns an execution proposal.

`POST /api/agent/evaluate-intent-with-allowance` also reads ERC-20 allowance from Mantle RPC and returns the correct next draft: approval calldata when allowance is low, or strategy calldata when allowance already covers the spend.

`POST /api/contracts/send-raw-transaction` relays a complete user-signed transaction through `eth_sendRawTransaction` when `MANTLE_RPC_URL` is configured.

Default testnet config targets Mantle Sepolia:

```env
MANTLE_RPC_URL=https://rpc.sepolia.mantle.xyz
MANTLE_CHAIN_ID=5003
```

Backend-signed actions are intentionally gated behind explicit `BACKEND_SIGNER_PRIVATE_KEY` and scoped execution policies.

Active intents are evaluated when activated and then by the worker's fast job loop. Each evaluation records an execution log containing condition results, actionability, transaction draft, and reasoning hash.

When token and strategy addresses are configured, Seer can emit runnable ERC-20 approval calldata for Mantle testnet:

```env
MANTLE_USDC_ADDRESS=
MANTLE_USDT_ADDRESS=
MANTLE_MNT_ADDRESS=
MANTLE_METH_ADDRESS=
SEER_APPROVED_STRATEGY_ADDRESS=
SEER_STRATEGY_DEPOSIT_FUNCTION=deposit(address,uint256)
```

For example, `accumulate 25 USDC` can produce an `erc20_approve` draft with `to=<USDC token>` and `data=approve(strategy, 25 USDC units)`.

`POST /api/contracts/erc20-allowance` checks existing token allowance through Mantle RPC. When allowance is already sufficient, Seer can skip the approval transaction and build a configured strategy call such as `deposit(address,uint256)` with `to=<strategy>` and `data=deposit(token, amount)`.

## Recurring Automation

Recurring execution uses the smart-account/session-key path.

```text
user owns smart account
user authorizes session key
Seer stores scoped session policy
worker/evaluate checks conditions
delegated execution enforces policy
Seer builds user-operation draft
AA provider/bundler submits the operation
```

Session policies include:

- smart account address
- session key address
- allowed assets
- allowed protocols
- allowed contracts
- transaction count limit
- spend limit
- expiry
- revocation

`POST /api/agent/intent/:intent_id/delegated-execute` only returns an executable user-operation draft when the intent is actionable and the active session policy allows it.

`AA_BUNDLER_URL` is reserved for the provider-specific ERC-4337 bundler integration.

When `AA_BUNDLER_URL` is configured, Seer can relay already-built ERC-4337 user operations:

```text
POST /api/contracts/send-user-operation
POST /api/contracts/user-operation-receipt
```

The backend still expects the smart-account provider/session-key flow to build and authorize the full user operation before relay.

## Authentication

Protected routes require a bearer session issued from wallet signature verification.

```text
POST /api/auth/challenge -> returns nonce + message
wallet signs message
POST /api/auth/verify -> returns bearer token
Authorization: Bearer <token>
```

The authenticated wallet must match the wallet in protected requests. This is enforced for intent creation/evaluation, wallet-specific intent reads, Arena entry and entry reads, identity generation/mint metadata, and signed transaction relay.

## Contracts

Foundry is configured with contracts in `contracts/`:

- `SeerArenaPoints`
- `SeerPredictionRegistry`
- `SeerIdentitySBT`
- `SeerIntentRegistry`

Run:

```bash
forge test
```

## Nansen Integration Path

The provider layer is built to accept a Nansen-backed implementation without changing product services.

Recommended mapping from `nansen-ai/nansen-skills`:

- `nansen-profiler` -> Wallet Service and Identity Service
- `nansen-smart-money` -> Signal Engine and Arena prediction facts
- `nansen-token` -> Token flow signals
- `nansen-portfolio` -> Portfolio positions and wallet summary

The current `NansenProvider` is scaffolded. Until live credentials and output schemas are wired, `MockProvider` provides deterministic Mantle-oriented demo intelligence.
