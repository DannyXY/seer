# Seer Backend Handover

## Current State

Seer now has a Rust/Axum backend, provider abstraction, Claude integration, Mantle RPC transaction relay boundaries, Foundry Solidity contracts, and a git history with verified checkpoints.

The backend currently supports:

- wallet signature authentication
- wallet summaries from provider data
- signal generation from provider facts
- Claude-backed explanations with deterministic fallback
- instant, recurring, conditional, and recurring-conditional intent parsing
- condition evaluation for TVL, APY, and risk score triggers
- condition provenance through `source_provider` and `source_captured_at`
- execution proposals with ERC-20 allowance metadata
- user-signed raw transaction relay through Mantle RPC
- ERC-4337 user-operation relay boundaries
- session-policy based delegated execution drafts
- worker evaluation of active executable intents
- Arena points and prediction lifecycle contracts
- Identity SBT and intent registry contracts

## Provider State

Implemented provider surfaces:

- Nansen `portfolio/defi-holdings` for wallet positions and wallet profile summaries
- Nansen `smart-money/holdings` for smart-money signal inputs
- DeFiLlama protocol TVL/APY fallback for protocol metrics
- MockProvider fallback for demo safety

Provider fallback order for protocol metrics:

```text
Nansen -> DeFiLlama -> MockProvider
```

Provider fallback for wallet and smart-money data:

```text
Nansen -> MockProvider
```

Still left:

- Nansen token holder endpoint mapping
- Nansen token screener endpoint mapping
- Nansen token flow endpoint mapping
- Nansen-native protocol metrics once the exact response schema is confirmed
- optional Allium provider integration if richer query workflows are needed

## Intent Execution State

The system supports these intent trigger families:

- `Instant`
- `Recurring`
- `Conditional`
- `RecurringConditional`

Example supported phrasing:

```text
Buy MNT now
Accumulate 25 USDC weekly into mETH
When mETH TVL climbs above 50M, accumulate 25 USDC weekly
When mETH risk is below 60 and TVL crosses 40M, execute within my policy
```

Execution is not wallet custody. The user either signs transactions manually, or grants a scoped smart-account/session-key policy.

Current execution path:

```text
intent text
  -> parsed intent
  -> provider condition evaluation
  -> execution proposal
  -> allowance check
  -> approval or strategy transaction draft
  -> user signature or session-policy user-operation draft
  -> Mantle RPC / bundler relay
```

## Protocol Execution State

Implemented safely:

- ERC-20 approval calldata builder
- configurable generic strategy deposit builder
- configurable protocol-specific strategy destination
- separate approval spender and execution target
- Lendle/Aave-style supply/deposit calldata:
  - `deposit(address,uint256,address,uint16)`
  - `supply(address,uint256,address,uint16)`

Still left before claiming full live protocol support:

- verify real Mantle testnet/mainnet addresses for each protocol and token
- configure protocol destination and spender env vars
- add real quote/slippage checks for swap-like routes
- add real Merchant Moe adapter only after selecting exact action:
  - swap
  - Liquidity Book add liquidity
  - classic AMM add liquidity
- verify Merchant Moe router ABI, route/path/bin parameters, quoter, and spender
- add withdrawal/reduce-position builders
- add transaction simulation or dry-run checks before surfacing executable drafts

Do not hardcode Merchant Moe execution until the exact route semantics are verified.

## Smart Account / Recurring Execution

Recurring execution cannot work from a normal EOA unless the user signs every transaction manually.

For actual recurring execution, use a smart account with a scoped session key:

```text
user owns smart account
user authorizes session key
Seer stores execution policy
worker evaluates active intents
policy checks pass
Seer returns/builds user-operation draft
smart-account provider signs/applies session-key rules
bundler relays user operation
```

What is still left:

- pick the smart-account provider stack
- wire the provider SDK that builds valid user operations
- validate session keys against the smart account on-chain
- optionally integrate paymaster support
- submit complete user operations through the configured bundler

Current backend relay boundary exists, but provider-specific smart-account signing/building is not complete.

## API And Worker Runtime

Yes: run the API and job process in parallel.

Recommended production layout:

```text
Render Web Service:
  APP_ROLE=api
  runs Axum API

Render Background Worker:
  APP_ROLE=worker
  runs scheduled jobs

Shared:
  DATABASE_URL
  REDIS_URL
  MANTLE_RPC_URL
  provider API keys
  deployed contract addresses
```

For local development, run two terminals:

```bash
cargo run
```

```bash
APP_ROLE=worker cargo run
```

Important note: the API role currently starts an internal MVP scheduler. For production with a separate worker, disable the internal scheduler or guard it with an env var such as:

```env
RUN_INTERNAL_JOBS=false
```

Recommended follow-up:

- add `RUN_INTERNAL_JOBS`
- default it to `true` for local MVP convenience
- set it to `false` on Render API service
- set `APP_ROLE=worker` for the Render worker service

This avoids duplicate intent evaluations and duplicate background jobs in production.

## Deployment Checklist

Required:

- `DATABASE_URL`
- `REDIS_URL`
- `MANTLE_RPC_URL`
- `MANTLE_CHAIN_ID`
- `CLAUDE_API_KEY`
- `NANSEN_API_KEY`
- deployed contract addresses

Recommended:

- `DEFILLAMA_ENABLED=true`
- `NANSEN_SMART_MONEY_CHAINS=ethereum,solana,base`
- `RUN_MIGRATIONS=true` during controlled deploys
- `RUN_INTERNAL_JOBS=false` on API service once implemented

Protocol execution env vars:

```env
MANTLE_USDC_ADDRESS=
MANTLE_USDT_ADDRESS=
MANTLE_MNT_ADDRESS=
MANTLE_METH_ADDRESS=

SEER_APPROVED_STRATEGY_ADDRESS=
SEER_APPROVED_STRATEGY_SPENDER_ADDRESS=
SEER_STRATEGY_DEPOSIT_FUNCTION=deposit(address,uint256)

SEER_LENDLE_STRATEGY_ADDRESS=
SEER_LENDLE_SPENDER_ADDRESS=
SEER_LENDLE_DEPOSIT_FUNCTION=deposit(address,uint256,address,uint16)
```

## Highest Priority Remaining Work

1. Add `RUN_INTERNAL_JOBS` so API and worker can run in parallel without duplicate background execution.
2. Choose and wire a smart-account/session-key provider SDK.
3. Validate one end-to-end Lendle supply action on Mantle testnet.
4. Add transaction simulation before returning executable drafts.
5. Add Nansen token holder, token screener, and token flow mappings.
6. Build a real Merchant Moe adapter only after route/ABI/quote details are verified.
7. Persist more runtime job results and signal snapshots to Postgres.
8. Add deployment manifests for Render API and Render Background Worker.

## Verification Baseline

Latest clean verification before this handover:

```text
cargo fmt
cargo check
cargo test
```

The last verified test count was 49 passing tests.

Before deploying, rerun:

```bash
cargo fmt
cargo check
cargo test
```

If contracts changed, also run Foundry tests or compile with `solc`.
