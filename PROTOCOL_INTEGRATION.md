# Protocol Integration Guide

## Overview

Seer integrates with Mantle protocols to execute user intents across DEXs, liquid staking, and RWA platforms. This document maps protocol operations to backend implementation.

**Network:** Mantle Sepolia Testnet (ChainID 5003)  
**RPC:** https://rpc.sepolia.mantle.xyz

---

## Supported Protocols

### 1. Agni Finance (Uniswap V3 Fork)

**Key Contracts (Testnet):**
- SwapRouter: `0xe2DB835566F8677d6889ffFC4F3304e8Df5Fc1df`
- QuoterV2: `0x49C8bb51C6bb791e8D6C31310cE0C14f68492991`
- NonfungiblePositionManager: `0xb04a19EF7853c52EDe6FBb28F8FfBecb73329eD7`
- Factory: `0x503Ca2ad7C9C70F4157d14CF94D3ef5Fa96D7032`

**Supported Operations:**

- **Swap (exactInputSingle)**
  - Input: `token_in`, `token_out`, `amount_in`, `fee_tier` (500|3000|10000)
  - Quote via: `QuoterV2.quoteExactInputSingle()`
  - Execute via: `SwapRouter.exactInputSingle()`
  - Backend: `AbiEncoder::encode_agni_exact_input_single()`

- **Add Liquidity (mint)**
  - Input: `token0`, `token1`, `fee`, `tickLower`, `tickUpper`, amounts
  - Calculate ticks from price range using: `priceToTick()` + `nearestUsableTick()`
  - Execute via: `NonfungiblePositionManager.mint()`
  - Returns: `tokenId` (NFT) — stored in `lp_positions` table

- **Remove Liquidity (decreaseLiquidity + collect)**
  - Input: `tokenId`, `liquidity` percentage
  - Two-step: call `decreaseLiquidity()` then `collect()`
  - Backend: Query `tokenId` from `lp_positions` table

- **Collect Fees**
  - Input: `tokenId`
  - Execute via: `NonfungiblePositionManager.collect()`

**Fee Tiers:**
- 500 (0.05%) — stable pairs
- 3000 (0.30%) — standard pairs
- 10000 (1.00%) — volatile pairs

### 2. Merchant Moe (Liquidity Book v2.2)

**Key Contracts (Mainnet):**
- LBRouter: `0x013e138EF6008ae5FDFDE29700e3f2Bc61d21E3a`
- LBQuoter: `0x501b8AFd35df20f531fF45F6f695793AC3316c85`
- LBFactory: `0xa6630671775c4EA2743840F9A5016dCf2A104054`

**Supported Operations:**

- **Swap (swapExactTokensForTokens)**
  - Input: `token_path`, `amount_in`, `bin_steps`, `versions`
  - Quote via: `LBQuoter.findBestPathFromAmountIn()`
  - Execute via: `LBRouter.swapExactTokensForTokens()`
  - Backend: `AbiEncoder::encode_moe_swap_exact_tokens_for_tokens()`

- **Add Liquidity (addLiquidity)**
  - Input: `tokenX`, `tokenY`, `binStep`, amounts, `deltaIds`, `distributionX/Y`
  - Bins: discrete price ranges (binStep in bps, e.g., 25 = 0.25%)
  - Fetch active bin: `LBPair.getActiveId()`
  - Execute via: `LBRouter.addLiquidity()`
  - Returns: `depositIds[]`, `liquidityMinted[]` — stored in `lp_positions` table

- **Remove Liquidity (removeLiquidity)**
  - Input: `tokenX`, `tokenY`, `binStep`, `binIds[]`, `amounts[]`
  - Query `binIds` from `lp_positions` table
  - Execute via: `LBRouter.removeLiquidity()`

**Bin Mechanics:**
- Price: `1.0001^(binId - 8388608)`
- Current bin (trading hub) determines price
- LPs distribute capital across bins using `deltaIds` (relative offsets from active)
- `distributionX/Y` controls percent allocation per bin

### 3. mETH Protocol (Liquid Staking)

**Operations:**
- **Stake ETH → mETH**
  - Input: `amount_eth`
  - Execute via: `mETH.stake(amount_eth)`
  - Backend: `AbiEncoder::encode_meth_stake()`

- **Unstake mETH → ETH**
  - Input: `amount_meth`
  - Execute via: `mETH.unstake(amount_meth)`

### 4. Ondo USDY (RWA Yield)

**Operations:**
- **Deposit USDT → USDY**
  - Input: `amount_usdt`
  - Earn yield on USDT deposits

- **Redeem USDY → USDT**
  - Input: `amount_usdy`

### 5. Fluxion Network (RWA Spot + RFQ)

**Status:** Mainnet launch December 2025. Dev docs in progress.

---

## Backend Architecture

### Intent Flow

```
User Intent (text)
    ↓
ParsedIntent (agent.rs)
    • action: swap | addLiquidity | removeLiquidity | collectFees | stake | unstake
    • target_assets: [USDT, WMNT, ...]
    • target_protocols: [Agni Finance, Merchant Moe, ...]
    • spend_amount: { amount: 1.0, asset: "USDT" }
    ↓
ExecutionProposal (execution.rs)
    • protocol_operation: Some(ProtocolOperation::AgniSwap { ... })
    • transaction_draft: Some(TransactionDraft { data: calldata, ... })
    ↓
AbiEncoder (abi_encoder.rs)
    • Encodes function parameters into 0x-prefixed calldata
    ↓
UserOperation or Signed Tx
    • Executed via Bundler or user signer
```

### Key Services

**AgentService** (`services/agent.rs`)
- Parses natural language intents
- Infers protocol, action, assets
- Creates execution policies

**ExecutionService** (`services/execution.rs`)
- Evaluates conditions
- Builds transaction drafts
- Validates delegation policies

**QuoterService** (`services/quoter.rs`)
- Calls QuoterV2 (Agni) for swap quotes
- Calls LBQuoter (Merchant Moe) for best swap paths
- Calculates slippage-adjusted minimums

**AbiEncoder** (`services/abi_encoder.rs`)
- Encodes Agni/Merchant Moe function calls
- Encodes ERC20 approvals
- Produces 0x-prefixed calldata

**Database** (`db/mod.rs`)
- Persists LP positions (Agni tokenIds, Merchant Moe binIds)
- Enables removal/collection operations

### Data Models

**ProtocolOperation** (`models/execution.rs`)
```rust
enum ProtocolOperation {
    AgniSwap { token_in, token_out, amount_in, fee_tier, amount_out_minimum },
    AgniAddLiquidity { token0, token1, fee, tick_lower, tick_upper, ... },
    MerchantMoeSwap { token_path, amount_in, amount_out_minimum, bin_steps },
    MerchantMoeAddLiquidity { token_x, token_y, bin_step, ... },
    MethStake { amount_eth },
    ...
}
```

**LpPosition** (`models/lp_position.rs`)
```rust
struct LpPosition {
    wallet_address,
    protocol: AgniFinance | MerchantMoe,
    agni_position: Option<{ token_id, token0, token1, fee, tick_lower, tick_upper }>,
    moe_position: Option<{ lb_pair, bin_ids, liquidity_minted, ... }>,
    tx_hash,
    created_at,
}
```

### Database Schema

**lp_positions** table:
- `id`: UUID
- `wallet_address`: user wallet
- `protocol`: "AgniFinance" | "MerchantMoe"
- `agni_token_id`, `agni_token0/1`, `agni_fee`, `agni_tick_lower/upper`: Agni position data
- `moe_lb_pair`, `moe_bin_ids[]`, `moe_liquidity_minted[]`: Merchant Moe position data
- `amount_x_added`, `amount_y_added`: actual amounts deposited
- `tx_hash`: execution hash for auditing
- Indexes on `wallet_address`, `protocol`, `tx_hash`

---

## Configuration

### Environment Variables

**Agni:**
```
SEER_AGNI_STRATEGY_ADDRESS=...
SEER_AGNI_SPENDER_ADDRESS=...     # SwapRouter address for approvals
SEER_AGNI_DEPOSIT_FUNCTION=...    # e.g., "swap(address,uint256)"
```

**Merchant Moe:**
```
SEER_MERCHANT_MOE_STRATEGY_ADDRESS=...
SEER_MERCHANT_MOE_SPENDER_ADDRESS=... # LBRouter for approvals
SEER_MERCHANT_MOE_DEPOSIT_FUNCTION=...
```

**Token Addresses (Testnet):**
```
MANTLE_USDC_ADDRESS=0x82a2eb46a64e4908bbc403854bc8aa699bf058e9
MANTLE_USDT_ADDRESS=0x3e163F861826C3f7878bD8fa8117A179d80731Ab
MANTLE_WMNT_ADDRESS=0xEa12Be2389c2254bAaD383c6eD1fa1e15202b52A
MANTLE_USDY_ADDRESS=...
MANTLE_CMETH_ADDRESS=...
```

---

## API Endpoints

### Intent Evaluation
- `POST /api/agent/evaluate-intent` — Parse and evaluate user intent
- `POST /api/agent/evaluate-intent-with-allowance` — Check ERC20 allowance before execution

### Position Management
- `POST /api/positions/agni` — Record Agni LP position (tokenId)
- `POST /api/positions/merchant-moe` — Record Merchant Moe LP position (binIds)
- `GET /api/positions/:wallet_address` — List user's LP positions

### Execution
- `POST /api/contracts/send-user-operation` — Submit ERC-4337 UserOperation
- `POST /api/contracts/send-raw-transaction` — Submit signed tx

---

## Next Steps

1. **Quote Integration:** Wire QuoterV2/LBQuoter RPC calls in `QuoterService`
2. **UserOperation Batching:** Batch approve + swap into single UserOp callData
3. **Fee Tier Logic:** Auto-select fee tier based on trading pair volatility
4. **Price Range Math:** Implement tick/bin calculation for LP ranges
5. **Slippage Handling:** Store slippage preferences in execution policy
6. **Event Indexing:** Listen for SwapRouter/LBRouter events for position tracking

---

*Last updated: June 2026*
