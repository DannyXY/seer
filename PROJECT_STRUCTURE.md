# Seer - Project Structure Overview

Complete guide to the Seer codebase organization.

## Root Directory

```
seer/
├── backend/              # Rust backend API
├── frontend/             # React web interface
├── contract/             # Solidity smart contracts
├── docs/                 # Documentation
├── scripts/              # Utility scripts
├── .env                  # Environment variables
├── README.md            # Project overview
├── SETUP.md             # Setup instructions
├── PROJECT_STRUCTURE.md # This file
├── PROTOCOL_INTEGRATION.md # Protocol API reference
└── render.yaml          # Render.com deployment config
```

## Backend (`backend/`)

Rust API server built with Axum, handling protocol integrations, account abstraction, and intent execution.

```
backend/
├── src/
│   ├── main.rs               # Entry point
│   ├── lib.rs               # Library root
│   ├── api/                 # HTTP endpoints
│   │   ├── mod.rs          # Router setup
│   │   ├── agent.rs        # Intent management endpoints
│   │   ├── arena.rs        # Prediction market endpoints
│   │   ├── contracts.rs    # Contract execution endpoints
│   │   ├── positions.rs    # LP position endpoints
│   │   ├── signals.rs      # Signal feed endpoints
│   │   ├── wallet.rs       # Wallet info endpoints
│   │   ├── identity.rs     # Identity/SBT endpoints
│   │   ├── settings.rs     # User settings endpoints
│   │   ├── health.rs       # Health check
│   │   └── auth*.rs        # Authentication
│   ├── services/            # Business logic
│   │   ├── mod.rs          # Service registry
│   │   ├── agent.rs        # Intent parsing & policy
│   │   ├── execution.rs    # Intent evaluation & drafting
│   │   ├── abi_encoder.rs  # Protocol call encoding
│   │   ├── quoter.rs       # Protocol quote service
│   │   ├── contracts.rs    # Smart contract RPC calls
│   │   ├── claude.rs       # Claude API integration
│   │   ├── signal_engine.rs# Signal generation
│   │   ├── data_provider/  # External data sources
│   │   ├── arena.rs        # Prediction logic
│   │   └── [auth, wallet, identity].rs
│   ├── models/              # Data structures
│   │   ├── agent.rs        # Intent, Policy, ParsedIntent
│   │   ├── execution.rs    # ExecutionProposal, ProtocolOperation
│   │   ├── lp_position.rs  # LP position tracking
│   │   ├── signals.rs      # Signal models
│   │   ├── arena.rs        # Prediction models
│   │   └── [wallet, identity, auth].rs
│   ├── db/                  # Database layer
│   │   └── mod.rs          # Persistence functions
│   ├── config/              # Configuration
│   │   └── mod.rs          # Settings from environment
│   ├── errors/              # Error handling
│   │   └── mod.rs
│   ├── jobs/                # Background jobs
│   │   └── mod.rs
│   └── telemetry/           # Logging & tracing
│       └── mod.rs
├── migrations/              # PostgreSQL migrations
│   ├── 001_init.sql        # Initial schema
│   ├── 002_job_runs.sql    # Job tracking tables
│   └── 003_lp_positions.sql # LP position schema
├── Cargo.toml              # Rust dependencies
├── Cargo.lock              # Dependency lock file
├── rust-toolchain.toml     # Rust version
└── Dockerfile              # Container image
```

### Key Features

- **Intent Parsing** (`services/agent.rs`) — NLP-based intent understanding
- **Protocol Integration** (`services/execution.rs`, `services/abi_encoder.rs`) — Agni, Merchant Moe, mETH, Ondo USDY, Fluxion
- **Quote Service** (`services/quoter.rs`) — QuoterV2 & LBQuoter integration
- **Account Abstraction** (`models/execution.rs`) — ERC-4337 UserOperation building
- **LP Position Tracking** (`models/lp_position.rs`, `db/mod.rs`) — Store & retrieve positions
- **Policy Engine** (`models/agent.rs`) — Execution scope & permission management
- **Data Providers** (`services/data_provider/`) — Nansen, DeFiLlama metrics

### API Overview

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/health` | GET | Health check |
| `/api/agent/parse-intent` | POST | Parse user intent |
| `/api/agent/evaluate-intent` | POST | Evaluate & propose |
| `/api/agent/:address/intents` | GET | List intents |
| `/api/positions/agni` | POST | Record Agni position |
| `/api/positions/:wallet` | GET | List LP positions |
| `/api/contracts/send-user-operation` | POST | Submit UserOp |
| `/api/contracts/send-raw-transaction` | POST | Submit signed tx |
| `/api/contracts/execution-readiness` | GET | Check readiness |
| `/api/arena/predictions` | GET | List predictions |
| `/api/signals` | GET | List signals |

## Frontend (`frontend/`)

React-based web interface for user interaction with the agent.

```
frontend/
├── src/
│   ├── app.jsx              # Root component
│   ├── pages/               # Page-level components
│   │   ├── landing.jsx      # Welcome page
│   │   ├── agent.jsx        # Intent creation
│   │   ├── agentscreen.jsx  # Agent dashboard
│   │   ├── arena.jsx        # Predictions
│   │   ├── identity.jsx     # User identity
│   │   ├── [*.css]          # Page styles
│   ├── components/          # Reusable components
│   │   ├── shell.jsx        # Layout wrapper
│   │   ├── cardrender.jsx   # Card UI
│   │   ├── signalfeed.jsx   # Signal feed
│   │   ├── settings.jsx     # Settings panel
│   │   ├── tweaks.jsx       # Feature toggles
│   │   ├── tweaks-panel.jsx # Tweaks UI
│   │   └── primitives.jsx   # UI primitives
│   ├── styles/              # Global styles
│   │   ├── styles.css       # Main styles
│   │   ├── dashboard.css    # Dashboard styles
│   │   └── screens.css      # Responsive design
│   └── utils/               # Utilities
│       └── data.jsx         # Data helpers
├── public/                  # Static assets
│   └── index.html          # HTML entry point
├── package.json            # Dependencies
├── README.md               # Frontend guide
└── serve.py               # Development server
```

### Key Features

- **Intent Interface** (`pages/agent.jsx`) — Create & manage intents
- **Agent Dashboard** (`pages/agentscreen.jsx`) — Monitor executions
- **Prediction Market** (`pages/arena.jsx`) — Place bets on outcomes
- **Signal Feed** (`components/signalfeed.jsx`) — View protocol signals
- **Identity Management** (`pages/identity.jsx`) — Manage user profile & SBT
- **Responsive Design** — Mobile-first CSS in `src/styles/`

### Development

```bash
# Install dependencies
npm install

# Start dev server (port 3000)
npm run dev

# Build for production
npm run build

# Run linter
npm run lint

# Format code
npm run format
```

## Smart Contracts (`contract/`)

Solidity contracts for on-chain governance, predictions, and identity.

```
contract/
├── SeerArenaPoints.sol      # Prediction market points token
├── SeerPredictionRegistry.sol # Prediction storage & validation
├── SeerIdentitySBT.sol      # Soul-bound token for identity
├── SeerIntentRegistry.sol   # On-chain intent tracking
└── [test, foundry.toml]     # Tests & configuration
```

### Contracts

| Contract | Purpose |
|----------|---------|
| **ArenaPoints** | ERC20 token for prediction rewards |
| **PredictionRegistry** | Storage & management of predictions |
| **IdentitySBT** | Non-transferable identity token (SBT) |
| **IntentRegistry** | On-chain intent hash & metadata |

### Deploy

```bash
cd contract
forge build
forge test
forge script scripts/Deploy.s.sol --rpc-url $RPC_URL --broadcast
```

## Documentation (`docs/`)

Technical and architectural documentation.

```
docs/
├── ARCHITECTURE.md       # System design overview
├── PROTOCOL_INTEGRATION.md # Protocol API reference (in root)
├── API.md               # REST API documentation
└── DATABASE.md          # Database schema guide
```

## Configuration

### Environment Variables (`.env`)

```
# Application
APP_ENV=development
APP_ROLE=api
PORT=10000

# Database
DATABASE_URL=postgresql://...
RUN_MIGRATIONS=true

# Mantle Network
MANTLE_RPC_URL=https://rpc.sepolia.mantle.xyz
MANTLE_CHAIN_ID=5003

# Account Abstraction
AA_BUNDLER_URL=https://api.pimlico.io/v2/mantle/rpc
AA_ENTRY_POINT_ADDRESS=0x0000000071727De22E5E9d4467Bb36353Cccb409

# Tokens
MANTLE_USDT_ADDRESS=0x3e163F861826C3f7878bD8fa8117A179d80731Ab
MANTLE_USDC_ADDRESS=0x82a2eb46a64e4908bbc403854bc8aa699bf058e9
[... more in .env file ...]

# Protocols
SEER_AGNI_STRATEGY_ADDRESS=0xe2DB835566F8677d6889ffFC4F3304e8Df5Fc1df
SEER_MERCHANT_MOE_STRATEGY_ADDRESS=0x013e138EF6008ae5FDFDE29700e3f2Bc61d21E3a

# AI/Data
CLAUDE_API_KEY=...
NANSEN_API_KEY=...
```

See `.env` file for complete list.

## Database

PostgreSQL schema managed by migrations in `backend/migrations/`:

- **agents_intents** — User intents
- **agent_execution_policies** — Authorization scopes
- **agent_execution_logs** — Execution history
- **lp_positions** — Tracked liquidity positions
- **signals** — Generated signals
- **job_runs** — Background job tracking

## Development Workflow

### 1. Setup

```bash
# Clone repo
git clone https://github.com/your-org/seer.git
cd seer

# Install dependencies
cd backend && cargo build
cd ../frontend && npm install
cd ../contract && forge install
```

### 2. Configure

```bash
cp .env.example .env
# Edit .env with your API keys and addresses
```

### 3. Run Locally

```bash
# Terminal 1 - Backend
cd backend
cargo run

# Terminal 2 - Frontend
cd frontend
npm start

# Terminal 3 - Database (if using local postgres)
# Just ensure postgres is running
```

### 4. Develop

- **Backend Changes** → Auto-reload with `cargo watch`
- **Frontend Changes** → Hot-reload with Vite
- **Contracts** → Test with `forge test`

### 5. Deploy

```bash
# Docker
docker-compose up

# Or individually
docker build backend -t seer-api
docker build frontend -t seer-web
docker run -p 10000:10000 seer-api
docker run -p 3000:3000 seer-web
```

## Important Files

| File | Purpose |
|------|---------|
| `backend/src/config/mod.rs` | All configuration loading |
| `backend/src/services/execution.rs` | Intent evaluation logic |
| `backend/src/services/abi_encoder.rs` | Protocol calldata encoding |
| `backend/src/db/mod.rs` | Database persistence |
| `frontend/src/pages/agent.jsx` | Main user interface |
| `contract/SeerIntentRegistry.sol` | On-chain tracking |
| `.env` | Environment configuration |
| `PROTOCOL_INTEGRATION.md` | Protocol API reference |

## Key Concepts

### Intent
User's natural language instruction to execute across protocols.

**Flow:** Parse → Evaluate Conditions → Build Proposal → Execute via AA

### Execution Policy
Scoped permissions for autonomous agent execution (session keys).

**Components:** Allowed protocols, assets, spending limit, transaction count

### Protocol Operation
Specific on-chain function call (swap, addLiquidity, mint, stake, etc.)

**Examples:**
- Agni: `exactInputSingle`, `mint`, `decreaseLiquidity`
- Merchant Moe: `swapExactTokensForTokens`, `addLiquidity`
- mETH: `stake`, `unstake`

### LP Position
Tracked liquidity positions for management and fee collection.

**Data:** Agni tokenIds, Merchant Moe binIds, amounts added, tx hash

### Signal
Market intelligence alert (TVL change, APY opportunity, risk indicator).

**Sources:** Nansen smart money, DeFiLlama metrics, on-chain analysis

## Next Steps

1. **Complete Setup** — Follow `SETUP.md`
2. **Read Protocols** — Review `PROTOCOL_INTEGRATION.md`
3. **Explore Code** — Start with `backend/src/main.rs`
4. **Test Flow** — POST to `/api/agent/evaluate-intent`
5. **Build UI** — Extend `frontend/src/pages/`
6. **Deploy** — Use Docker or cloud provider

---

For detailed instructions, see:
- `SETUP.md` — Getting started
- `PROTOCOL_INTEGRATION.md` — Protocol API reference
- `backend/README.md` — Backend details
- `frontend/README.md` — Frontend guide
- `docs/ARCHITECTURE.md` — System design
