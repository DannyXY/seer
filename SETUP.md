# Seer - Project Setup Guide

This guide explains the project structure and how to set up each component.

## Project Structure

```
seer/
├── backend/                    # Rust backend (Axum API server)
│   ├── src/                    # Rust source code
│   ├── migrations/             # Database migrations (PostgreSQL)
│   ├── Cargo.toml             # Rust dependencies
│   └── Dockerfile             # Container setup
├── frontend/                   # React frontend
│   ├── src/                   # React components & pages
│   ├── package.json           # Node dependencies
│   └── [CSS/assets]           # Styling & static files
├── contract/                   # Smart contracts (Solidity/Foundry)
│   ├── src/                   # Contract source code
│   ├── test/                  # Contract tests
│   └── foundry.toml           # Foundry configuration
├── docs/                       # Documentation
├── scripts/                    # Utility scripts
├── .env                        # Environment variables (development)
└── README.md                   # Project overview
```

## Environment Setup

### 1. Copy Environment Variables

All required environment variables are pre-configured in `.env`:

```bash
cd seer
cat .env
```

**Key sections to configure:**

- **MANTLE_RPC_URL** — Already set to Mantle Sepolia testnet
- **DATABASE_URL** — PostgreSQL connection (requires local postgres)
- **CLAUDE_API_KEY** — Add your Anthropic API key
- **NANSEN_API_KEY** — Add your Nansen Analytics key
- **Protocol Addresses** — Testnet addresses already filled for Agni & Merchant Moe
- **AA Configuration** — Bundler/Paymaster URLs ready for Pimlico

### 2. Database Setup

```bash
# Install PostgreSQL (if not installed)
# macOS:
brew install postgresql@15

# Start PostgreSQL
brew services start postgresql@15

# Create database
createdb seer

# Verify connection
psql -U postgres -d seer -c "SELECT 1;"
```

### 3. Backend Setup

```bash
cd backend

# Install Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Build backend
cargo build

# Run migrations
RUN_MIGRATIONS=true cargo run

# Backend will start on http://localhost:10000
```

**Key API Endpoints:**

- `GET /api/health` — Health check
- `POST /api/agent/parse-intent` — Parse user intent
- `POST /api/agent/evaluate-intent` — Evaluate & propose execution
- `POST /api/positions/agni` — Record Agni LP position
- `GET /api/positions/:wallet` — List user's LP positions
- `POST /api/contracts/send-user-operation` — Submit ERC-4337 UserOp
- `POST /api/contracts/send-raw-transaction` — Submit signed tx

### 4. Frontend Setup

```bash
cd frontend

# Install Node.js (if not installed)
# Option 1: macOS with Homebrew
brew install node

# Option 2: Download from https://nodejs.org/

# Install dependencies
npm install

# Start development server
npm start

# Frontend will be available at http://localhost:3000
```

### 5. Smart Contracts Setup

```bash
cd contract

# Install Foundry (if not installed)
curl -L https://foundry.paradigm.xyz | bash
~/.foundry/bin/foundryup

# Build contracts
forge build

# Run tests
forge test

# Deploy to Mantle Sepolia
forge script scripts/Deploy.s.sol --rpc-url https://rpc.sepolia.mantle.xyz \
  --broadcast \
  --verify
```

## Configuration Details

### Protocols Configured

#### Agni Finance (DEX)
- **SwapRouter:** `0xe2DB835566F8677d6889ffFC4F3304e8Df5Fc1df`
- **QuoterV2:** `0x49C8bb51C6bb791e8D6C31310cE0C14f68492991`
- **Supports:** Swaps, add/remove liquidity, collect fees
- **Fee tiers:** 500 (0.05%), 3000 (0.3%), 10000 (1%)

#### Merchant Moe (DEX)
- **LBRouter:** `0x013e138EF6008ae5FDFDE29700e3f2Bc61d21E3a`
- **LBQuoter:** `0x501b8AFd35df20f531fF45F6f695793AC3316c85`
- **Supports:** Swaps, add/remove liquidity (bin-based)

#### mETH Protocol (Liquid Staking)
- **Operations:** Stake ETH → mETH, unstake mETH → ETH

#### Ondo USDY (RWA)
- **Operations:** Deposit USDT → USDY, redeem USDY → USDT

### Account Abstraction (ERC-4337)

**Bundler:** Pimlico  
**Entry Point:** `0x0000000071727De22E5E9d4467Bb36353Cccb409`  
**Paymaster:** Configured for gas sponsorship

**Supported Smart Accounts:**
- Kernel (ZeroDev) — modular, session-key enabled
- Safe — battle-tested, multi-sig support

### AI/Data Integration

**Claude API** — Intent parsing & reasoning  
**Nansen Analytics** — Smart money tracking  
**DeFiLlama** — Protocol metrics & yields  

## Running the Full Stack

### Option 1: Local Development (3 terminals)

**Terminal 1 - Backend:**
```bash
cd backend
cargo run
# Starts on http://localhost:10000
```

**Terminal 2 - Frontend:**
```bash
cd frontend
npm start
# Starts on http://localhost:3000
```

**Terminal 3 - Database (if using local postgres):**
```bash
# Postgres runs automatically via Homebrew/services
# Monitor logs:
tail -f /usr/local/var/log/postgres.log
```

### Option 2: Docker (Single Command)

```bash
# Build all images
docker-compose build

# Run stack
docker-compose up

# Backend:  http://localhost:10000
# Frontend: http://localhost:3000
# Postgres: localhost:5432
```

## Testing

### Backend Unit Tests

```bash
cd backend
cargo test
```

### Backend Integration Tests

```bash
cd backend
# Requires DATABASE_URL and MANTLE_RPC_URL set
cargo test -- --ignored --test-threads=1
```

### Frontend Tests

```bash
cd frontend
npm test
```

### Smart Contract Tests

```bash
cd contract
forge test
```

## Troubleshooting

**Backend won't start:**
- Check `DATABASE_URL` is correct
- Verify PostgreSQL is running: `psql -U postgres -d seer -c "SELECT 1;"`
- Check port 10000 is available: `lsof -i :10000`

**Frontend can't connect to backend:**
- Verify backend is running on http://localhost:10000
- Check CORS is enabled (it is, by default)
- Clear browser cache: Cmd+Shift+Del

**Migrations fail:**
- Ensure `RUN_MIGRATIONS=true` in .env
- Delete and recreate database: `dropdb seer && createdb seer`

**Contract deployment fails:**
- Verify private key is correct
- Check testnet RPC is accessible: `curl https://rpc.sepolia.mantle.xyz`
- Ensure account has testnet MNT for gas

## Next Steps

1. **Configure AI Key:** Add `CLAUDE_API_KEY` to .env for intent parsing
2. **Test Agent Flow:** POST to `/api/agent/evaluate-intent` with a swap intent
3. **Record Positions:** Use `/api/positions/agni` to track LP positions
4. **Build UI:** Expand frontend components in `frontend/src`
5. **Integrate Quotes:** Wire `QuoterService` to real RPC calls in backend
6. **Deploy:** Use Dockerfile or Render.yaml for production

## Documentation

- **PROTOCOL_INTEGRATION.md** — Detailed protocol API reference
- **ARCHITECTURE.md** — System design & data flow
- **README.md** — Project overview & vision

---

**Questions?** Open an issue or check docs/ directory.
