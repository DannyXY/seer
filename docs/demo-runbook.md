# Demo Runbook

## Start API

```bash
cp .env.example .env
cargo run
```

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

## Evaluate Actionable Intent

```bash
curl -X POST http://localhost:10000/api/agent/evaluate-intent \
  -H 'content-type: application/json' \
  -d '{
    "wallet_address": "0x1234567890123456789012345678901234567890",
    "raw_intent": "When mETH TVL climbs above 40M and my risk score is below 60, accumulate 25 USDC weekly into mETH"
  }'
```

Expected shape:

```text
actionable = true
conditions include observed provider values
transaction_draft describes the user-signed Mantle testnet action
```

Set `MANTLE_USDC_ADDRESS` and `SEER_APPROVED_STRATEGY_ADDRESS` to receive a concrete `erc20_approve` transaction draft for USDC accumulation intents.

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

## Mantle RPC Readiness

```bash
curl http://localhost:10000/api/contracts/readiness
```

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
