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
- Mantle RPC transaction simulation before surfacing concrete allowance-aware drafts
- user-signed raw transaction relay through Mantle RPC
- ERC-4337 user-operation relay boundaries
- session-policy based delegated execution drafts
- worker evaluation of active executable intents
- Postgres persistence for generated signal snapshots and fast worker job summaries
- Arena points and prediction lifecycle contracts
- Identity SBT and intent registry contracts

## Provider State

Implemented provider surfaces:

- Nansen `portfolio/defi-holdings` for wallet positions and wallet profile summaries
- Nansen `smart-money/holdings` for smart-money signal inputs
- Nansen `token-screener` for token-level smart-money signal inputs
- Nansen Token God Mode `tgm/flows` for token flow signals
- Nansen Token God Mode `tgm/holders` as token holder fallback summaries
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

- validate session keys against the smart account on-chain
- optionally integrate paymaster support
- validate one complete Safe ERC-4337 user operation through the configured bundler

Selected stack:

```text
AA_PROVIDER_STACK=safe-4337-relay-kit
AA_ENTRY_POINT_ADDRESS=<Safe4337 EntryPoint>
AA_BUNDLER_URL=<ERC-4337 bundler>
AA_PAYMASTER_URL=<optional paymaster>
```

Current backend relay boundary validates session-policy addresses and provider-built user-operation shape. Provider-specific Safe session-key signing/building remains outside the Rust backend and must produce a complete signed user operation before Seer relays it.

`GET /api/contracts/readiness` exposes `live_validation.safe_user_operation` and `live_validation.lendle_supply` to show missing env/config before attempting live Safe or Lendle validation.

Use `scripts/live-validation-smoke.sh` against a running API to print readiness and optionally enforce `REQUIRE_SAFE_READY=1` or `REQUIRE_LENDLE_READY=1`.

## API And Worker Runtime

Yes: run the API and job process in parallel.

Recommended production layout:

```text
Render Web Service:
  APP_ROLE=api
  RUN_INTERNAL_JOBS=false
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

The API role starts an internal MVP scheduler by default for local convenience. Production API services should disable it when a separate worker is running:

```env
RUN_INTERNAL_JOBS=false
```

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
- `RUN_INTERNAL_JOBS=false` on API service when a separate worker is running

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

1. Validate one complete Safe ERC-4337 user operation through the configured bundler.
2. Validate one end-to-end Lendle supply action on Mantle testnet.
3. Build a real Merchant Moe adapter only after route/ABI/quote details are verified.

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
