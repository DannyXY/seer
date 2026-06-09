# Seer

Seer is a wallet intelligence and prediction system for the Mantle Turing Test Hackathon. It uses Mantle on-chain data as a core data source — native MNT balances are fetched live via `eth_getBalance` on the Mantle RPC, Arena points and claimed status are read via `eth_call` against deployed contracts, and agent intent/policy hashes are anchored on-chain via `SeerIntentRegistry`.

## Deployed Contracts (Mantle Sepolia Testnet)

| Contract               | Address                                      |
| ---------------------- | -------------------------------------------- |
| SeerArenaPoints        | `0x2B8cCC79007a66053eA081786A886174CD548eEd` |
| SeerPredictionRegistry | `0x1E255E1C5A18d79F4ee1FF7a5BC9dB7e542e68e8` |
| SeerIdentitySBT        | `0x1B46bb805a6707449B27C95175D0a2ff07Cb6BA2` |
| SeerIntentRegistry     | `0x71cE98dA05B66a19c1894d8d2ea0b81600D461D9` |

All four contracts are verified on [Mantle Sepolia Explorer](https://explorer.sepolia.mantle.xyz).

Frontend URL: https://seer-mantle.onrender.com/
Backend URL: https://seer-api-7mlt.onrender.com

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

## Durable State

When `DATABASE_URL` is configured, Seer persists generated signal snapshots, created agent intents, execution policies, agent execution logs, and worker job-run summaries to PostgreSQL. If Postgres is not configured, the same API remains usable with in-memory state for local demo reliability.

Set `RUN_MIGRATIONS=true` to run SQLx migrations on startup. It defaults to `false` so copying `.env.example` does not break local demo runs when Postgres is not available.

## Important Execution Rule

Seer does not require broad wallet custody.

Agent execution is modeled as:

```text
intent -> parsed trigger -> execution policy -> user approval or scoped delegated permission -> execution log
```

Seer does not store user funds. User funds remain in the user's wallet or smart account until a user-signed transaction or authorized session-key user operation executes against the destination protocol contract.

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

Named Mantle destinations such as Merchant Moe, Lendle, Agni Finance, and mETH Protocol can be configured with protocol-specific strategy addresses and deposit function signatures. Without those real protocol addresses/ABIs, Seer can evaluate and recommend but cannot honestly claim protocol-specific execution.

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
GET  /api/contracts/execution-readiness
POST /api/contracts/send-raw-transaction
POST /api/contracts/send-user-operation
POST /api/contracts/simulate-transaction
POST /api/contracts/user-operation-receipt
POST /api/contracts/erc20-allowance
```

## Live Action Path

Seer should make intelligence actionable on Mantle testnet without taking broad wallet custody.

```text
intent -> condition evaluation -> execution proposal -> user signs transaction -> Seer relays signed tx to Mantle RPC
```

`POST /api/agent/evaluate-intent` evaluates parsed conditions against provider facts and returns an execution proposal.

`POST /api/agent/evaluate-intent-with-allowance` also reads ERC-20 allowance from Mantle RPC and simulates concrete transaction drafts with `eth_call` before returning them. If simulation fails, Seer returns a non-runnable `simulation_failed` draft instead of executable calldata.

`POST /api/contracts/send-raw-transaction` relays a complete user-signed transaction through `eth_sendRawTransaction` when `MANTLE_RPC_URL` is configured.

Default testnet config targets Mantle Sepolia:

```env
MANTLE_RPC_URL=https://rpc.sepolia.mantle.xyz
MANTLE_CHAIN_ID=5003
```

Backend-signed actions are intentionally gated behind explicit `BACKEND_SIGNER_PRIVATE_KEY` and scoped execution policies.

Active intents are evaluated when activated and then by the worker's fast job loop. Each evaluation records an execution log containing condition results, actionability, transaction draft, and reasoning hash. The fast job also persists generated signal snapshots and a `job_runs` summary when Postgres is configured.

When token and strategy addresses are configured, Seer can emit runnable ERC-20 approval calldata for Mantle testnet:

```env
MANTLE_USDC_ADDRESS=
MANTLE_USDT_ADDRESS=
MANTLE_MNT_ADDRESS=
MANTLE_METH_ADDRESS=
SEER_APPROVED_STRATEGY_ADDRESS=
SEER_APPROVED_STRATEGY_SPENDER_ADDRESS=
SEER_STRATEGY_DEPOSIT_FUNCTION=deposit(address,uint256)
```

For example, `accumulate 25 USDC` can produce an `erc20_approve` draft with `to=<USDC token>` and `data=approve(spender, 25 USDC units)`. If `SEER_APPROVED_STRATEGY_SPENDER_ADDRESS` is not set, the strategy address is used as the spender.

`POST /api/contracts/erc20-allowance` checks existing token allowance through Mantle RPC. When allowance is already sufficient, Seer can skip the approval transaction and build a configured strategy call such as `deposit(address,uint256)` with `to=<strategy>` and `data=deposit(token, amount)`.

`POST /api/contracts/simulate-transaction` dry-runs a draft transaction through Mantle RPC using `eth_call`. It is useful for clients and operators that want to check calldata before asking a wallet or smart-account provider to sign.

`GET /api/contracts/execution-readiness` reports configured token addresses and named protocol destinations, including Merchant Moe, Lendle, Agni Finance, and mETH Protocol. Readiness includes both the execution target and approval spender for each protocol.

`GET /api/contracts/readiness` includes `live_validation.safe_user_operation` and `live_validation.lendle_supply`. Check those before live execution; each object reports `ready`, missing env/config fields, and the next validation step.

For a repeatable local check against a running API:

```bash
scripts/live-validation-smoke.sh
```

Set `REQUIRE_SAFE_READY=1` or `REQUIRE_LENDLE_READY=1` to make the script fail when that live path is not fully configured.

## Recurring Automation

Recurring execution uses the smart-account/session-key path.

```text
user owns smart account
user authorizes session key
Seer stores scoped session policy
worker/evaluate checks conditions
delegated execution enforces policy
Seer builds user-operation draft
Safe ERC-4337 provider signs/builds the complete user operation
bundler submits the operation
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

`AA_PROVIDER_STACK` defaults to `safe-4337-relay-kit`, matching Safe Smart Accounts with Safe4337Module support and provider-built user operations. `AA_ENTRY_POINT_ADDRESS` and `AA_BUNDLER_URL` must be configured before Seer reports user-operation relay readiness. `AA_PAYMASTER_URL` is optional for sponsored execution.

When the bundler and entry point are configured, Seer can relay already-built ERC-4337 user operations:

```text
POST /api/contracts/send-user-operation
POST /api/contracts/user-operation-receipt
```

The backend validates session-policy addresses and provider-built user-operation shape before relay. It still expects the Safe ERC-4337 provider/session-key flow to build, authorize, and sign the full user operation before Seer sends it to the bundler.

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

If Foundry is not installed, compile with npm solc:

```bash
npx --yes solc@0.8.24 --bin --abi contracts/SeerArenaPoints.sol contracts/SeerPredictionRegistry.sol contracts/SeerIdentitySBT.sol contracts/SeerIntentRegistry.sol -o /tmp/seer-solc-out
```

Arena entries settle per prediction entry. `SeerPredictionRegistry.settleEntry` uses `SeerArenaPoints.settleLockedPoints` so resolving one entry does not unlock unrelated points from other active predictions.

## Nansen Integration Path

The provider layer is built to accept a Nansen-backed implementation without changing product services.

Recommended mapping from `nansen-ai/nansen-skills`:

- `nansen-profiler` -> Wallet Service and Identity Service
- `nansen-smart-money` -> Signal Engine and Arena prediction facts
- `nansen-token` -> Token flow signals
- `nansen-portfolio` -> Portfolio positions and wallet summary

The current `NansenProvider` wires Nansen's `portfolio/defi-holdings` endpoint for wallet positions and wallet profile summaries, `smart-money/holdings` and `token-screener` for Signal Engine smart-money movement inputs, plus Token God Mode `tgm/flows` and `tgm/holders` for token-flow signals when Mantle token addresses are configured. Protocol TVL/APY condition checks can use DeFiLlama through `DEFILLAMA_ENABLED=true`, falling back to mock data if unavailable. Wallet transaction and Nansen-native protocol metric methods still fall back through `ProviderRegistry` until their live response schemas are mapped. `MockProvider` remains the deterministic Mantle-oriented demo fallback.
