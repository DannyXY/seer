# Demo Runbook

## Start API

```bash
cp .env.example .env
cargo run
```

For live Claude reasoning, set:

```env
CLAUDE_API_KEY=sk-ant-...
CLAUDE_MODEL=claude-sonnet-4-20250514
```

Without `CLAUDE_API_KEY`, the API still runs with deterministic fallback explanations.

## Start Worker

```bash
APP_ROLE=worker cargo run
```

## Health

```bash
curl http://localhost:10000/api/health
```

## Authenticate Wallet

```bash
curl -X POST http://localhost:10000/api/auth/challenge \
  -H 'content-type: application/json' \
  -d '{ "wallet_address": "0x1234567890123456789012345678901234567890" }'
```

Sign the returned `message` with the wallet, then verify:

```bash
curl -X POST http://localhost:10000/api/auth/verify \
  -H 'content-type: application/json' \
  -d '{
    "wallet_address": "0x1234567890123456789012345678901234567890",
    "nonce": "<nonce>",
    "message": "<message>",
    "signature": "<wallet_signature>"
  }'
```

Use the returned token on protected routes:

```bash
Authorization: Bearer <token>
```

## Generate Signals

```bash
curl http://localhost:10000/api/signals
```

## Parse Conditional Intent

```bash
curl -X POST http://localhost:10000/api/agent/parse-intent \
  -H 'content-type: application/json' \
  -d '{
    "wallet_address": "0x1234567890123456789012345678901234567890",
    "raw_intent": "When mETH TVL climbs above 50M and my risk score is below 60, accumulate 25 USDC weekly into mETH"
  }'
```

Expected shape:

```text
trigger.mode = RecurringConditional
conditions include tvl/risk-style checks
requires_user_signature = true
```

Supported trigger phrasing includes `climbs to`, `reaches`, `crosses`, `exceeds`, `at least`, `below`, `under`, `at most`, `recurring`, and `recurrent`.

## Evaluate Actionable Intent

```bash
curl -X POST http://localhost:10000/api/agent/evaluate-intent \
  -H 'authorization: Bearer <token>' \
  -H 'content-type: application/json' \
  -d '{
    "wallet_address": "0x1234567890123456789012345678901234567890",
    "raw_intent": "When mETH TVL climbs above 40M and my risk score is below 60, accumulate 25 USDC weekly into mETH"
  }'
```

Expected shape:

```text
actionable = true
conditions include observed provider values, source provider, and capture time
allowance_check identifies the token/spender Seer expects for approval
transaction_draft describes the user-signed Mantle testnet action
```

Set `MANTLE_USDC_ADDRESS` and `SEER_APPROVED_STRATEGY_ADDRESS` to receive a concrete `erc20_approve` transaction draft for USDC accumulation intents. Set `SEER_APPROVED_STRATEGY_SPENDER_ADDRESS` when the allowance spender differs from the execution target. Set `SEER_STRATEGY_DEPOSIT_FUNCTION` to the selected protocol's ABI signature, defaulting to `deposit(address,uint256)`, so Seer can draft the next strategy call after allowance is sufficient.

## Evaluate With Live Allowance

```bash
curl -X POST http://localhost:10000/api/agent/evaluate-intent-with-allowance \
  -H 'authorization: Bearer <token>' \
  -H 'content-type: application/json' \
  -d '{
    "wallet_address": "0x1234567890123456789012345678901234567890",
    "raw_intent": "When mETH TVL climbs above 40M and my risk score is below 60, accumulate 25 USDC weekly into mETH",
    "owner_address": "0xWalletOrSmartAccount"
  }'
```

This evaluates provider conditions and reads ERC-20 allowance from Mantle RPC in one call. If the intent asset and protocol destination are configured, Seer derives the token and spender from the parsed intent. If they are not configured, include `token_address` and `spender_address` explicitly. The returned proposal gives the next transaction draft: approval if allowance is low, strategy execution if allowance is sufficient.

## Check ERC-20 Allowance

```bash
curl -X POST http://localhost:10000/api/contracts/erc20-allowance \
  -H 'authorization: Bearer <token>' \
  -H 'content-type: application/json' \
  -d '{
    "token_address": "0xToken",
    "owner_address": "0xWalletOrSmartAccount",
    "spender_address": "0xStrategy"
  }'
```

If allowance covers the intended spend amount, Seer can draft the configured strategy call instead of another approval.

## Mantle RPC Readiness

```bash
curl http://localhost:10000/api/contracts/readiness
```

## Execution Destination Readiness

```bash
curl http://localhost:10000/api/contracts/execution-readiness
```

Use this before demoing named protocol execution. A protocol is only ready for strategy drafts when its strategy address is configured. The readiness response includes both `strategy_address` and `approval_spender_address`, because some Mantle DeFi routes approve a router/spender but execute through a different target.

If the intent names Merchant Moe, Lendle, or Agni Finance and that destination is not configured, Seer returns a non-runnable recommendation instead of falling back to a generic strategy address.

For Lendle-style supply actions, configure the LendingPool as the destination and use the Aave-style deposit signature:

```env
SEER_LENDLE_STRATEGY_ADDRESS=<Lendle LendingPool>
SEER_LENDLE_SPENDER_ADDRESS=<Lendle LendingPool>
SEER_LENDLE_DEPOSIT_FUNCTION=deposit(address,uint256,address,uint16)
```

Seer also supports `supply(address,uint256,address,uint16)` for Aave-v3-style adapters. Merchant Moe should stay configurable-only until the exact swap or Liquidity Book route, router ABI, quote source, and slippage constraints are verified.

## Relay User-Signed Transaction

```bash
curl -X POST http://localhost:10000/api/contracts/send-raw-transaction \
  -H 'content-type: application/json' \
  -d '{ "signed_transaction": "0x..." }'
```

This endpoint expects a complete signed transaction. It does not sign on behalf of the user.

## Activate Intent

After creating an intent with an authenticated request, activate it:

```bash
curl -X POST http://localhost:10000/api/agent/intent/<intent_id>/activate \
  -H 'authorization: Bearer <token>'
```

Activation evaluates the intent immediately and stores an execution log. Worker ticks continue evaluating active recurring and conditional intents.

## Create Session-Key Policy

```bash
curl -X POST http://localhost:10000/api/agent/intent/<intent_id>/session-policy \
  -H 'authorization: Bearer <token>' \
  -H 'content-type: application/json' \
  -d '{
    "smart_account_address": "0xSmartAccount",
    "session_key_address": "0xSessionKey",
    "allowed_assets": ["mETH", "USDC"],
    "allowed_protocols": ["mETH Protocol"],
    "allowed_contracts": [],
    "max_spend_usd": 100,
    "max_transaction_count": 4,
    "expires_in_days": 30
  }'
```

## Delegated Execute

```bash
curl -X POST http://localhost:10000/api/agent/intent/<intent_id>/delegated-execute \
  -H 'authorization: Bearer <token>'
```

This evaluates the active intent, enforces the session-key policy, records an execution log, and returns a user-operation draft when executable.

## Submit User Operation

After a smart-account provider builds and signs a complete user operation:

```bash
curl -X POST http://localhost:10000/api/contracts/send-user-operation \
  -H 'authorization: Bearer <token>' \
  -H 'content-type: application/json' \
  -d '{
    "entry_point": "0xEntryPointAddress",
    "user_operation": {
      "sender": "0xSmartAccount",
      "nonce": "0x0",
      "callData": "0x...",
      "signature": "0x..."
    }
  }'
```

Then poll the receipt:

```bash
curl -X POST http://localhost:10000/api/contracts/user-operation-receipt \
  -H 'authorization: Bearer <token>' \
  -H 'content-type: application/json' \
  -d '{ "user_operation_hash": "0x..." }'
```

## Generate Identity

```bash
curl -X POST http://localhost:10000/api/identity/0x1234567890123456789012345678901234567890/generate
```

## Arena

```bash
curl http://localhost:10000/api/arena/predictions
curl http://localhost:10000/api/arena/leaderboard
```
