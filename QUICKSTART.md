# Seer — Quick Start Guide

Get Seer running locally in under 10 minutes.

## Prerequisites

- **Node.js 18+** — [Download](https://nodejs.org)
- **PostgreSQL** — [Download](https://www.postgresql.org/download) or use Docker: `docker run -d -p 5432:5432 -e POSTGRES_PASSWORD=postgres postgres`
- **Redis** — [Download](https://redis.io/download) or use Docker: `docker run -d -p 6379:6379 redis`
- **Rust** (for backend) — [Install](https://rustup.rs/)

## 1. Backend Setup (Rust API)

```bash
# Navigate to backend
cd backend

# Set up environment
cp ../.env.example .env.local  # Or use the existing .env

# Run database migrations
cargo sqlx migrate run

# Start the backend server (runs on port 10000)
cargo run
```

**Backend will be available at:** `http://localhost:10000`

## 2. Frontend Setup (React + Vite)

### Get Credentials

You'll need two things to enable authentication:

#### A. Privy App ID
1. Go to [https://privy.io](https://privy.io) and sign up
2. Create a new application
3. Copy your **App ID** from the dashboard
4. Enable login methods: Email, Google, GitHub, Twitter, Discord

#### B. Pimlico API Key
1. Go to [https://pimlico.io](https://pimlico.io) and sign up
2. Create a new project (select **Mantle Sepolia** as the network)
3. Copy your **API Key** from project settings

### Install & Configure

```bash
# Navigate to frontend
cd frontend

# Install dependencies
npm install

# Create environment file
cat > .env.development.local << EOF
VITE_PRIVY_APP_ID=YOUR_PRIVY_APP_ID_HERE
VITE_PIMLICO_API_KEY=YOUR_PIMLICO_API_KEY_HERE
VITE_API_BASE=http://localhost:10000
EOF

# Replace YOUR_PRIVY_APP_ID_HERE and YOUR_PIMLICO_API_KEY_HERE with actual values

# Start development server (runs on port 5173)
npm run dev
```

**Frontend will be available at:** `http://localhost:5173`

## 3. Test the Full Flow

1. **Open** `http://localhost:5173` in your browser
2. **Click** "Connect Wallet" on the landing page
3. **Sign in** with email or social provider (Privy modal)
4. **Approve** smart account creation (Pimlico will deploy it)
5. **Enter app** dashboard once authenticated

## File Structure

```
seer/
├── backend/                 # Rust Axum API
│   ├── src/
│   │   ├── main.rs         # Entry point
│   │   ├── api/            # Route handlers
│   │   ├── services/       # Business logic
│   │   ├── models/         # Data models
│   │   └── db/             # Database layer
│   └── Cargo.toml
├── frontend/                # React + Vite
│   ├── src/
│   │   ├── main.jsx        # Entry point
│   │   ├── app.jsx         # Root component
│   │   ├── pages/          # Page components
│   │   ├── components/     # Reusable components
│   │   ├── utils/          # Utilities
│   │   └── styles/         # Global styles
│   ├── package.json
│   └── vite.config.js
├── contract/                # Solidity smart contracts
│   ├── contracts/
│   └── foundry.toml
└── README.md
```

## Key Features Implemented

✅ **Privy Social Login** — Sign in with email, Google, GitHub, Twitter, Discord
✅ **Smart Accounts** — Automatic account creation on first login via Pimlico
✅ **Signal Feed** — Real-time on-chain signal detection from Mantle protocols
✅ **Agent Intent** — AI-powered intent parsing and execution
✅ **Arena/Predictions** — Make predictions and earn points
✅ **Identity Card** — User on-chain identity & reputation tracking
✅ **LP Position Management** — Track liquidity positions in Agni & Merchant Moe

## API Endpoints (Backend)

| Method | Endpoint | Purpose |
|--------|----------|---------|
| POST | `/api/auth/challenge` | Get signing challenge |
| POST | `/api/auth/verify` | Verify signature & get session |
| GET | `/api/signals` | Fetch all signals |
| POST | `/api/agent/parse-intent` | Parse user intent text |
| POST | `/api/agent/evaluate-intent` | Evaluate intent & get proposal |
| POST | `/api/agent/create-intent` | Create executable intent |
| GET | `/api/positions/:address` | Get user's LP positions |
| POST | `/api/positions/agni` | Record Agni position |
| POST | `/api/positions/merchant-moe` | Record Merchant Moe position |

See `PROTOCOL_INTEGRATION.md` for full API documentation.

## Troubleshooting

### Backend won't start
```bash
# Check PostgreSQL is running
psql -U postgres -d seer -c "SELECT 1"

# Check Redis is running
redis-cli ping  # Should return PONG

# Run migrations
cargo sqlx migrate run

# Check port 10000 is free
lsof -i :10000
```

### Frontend won't start
```bash
# Clear cache
rm -rf node_modules package-lock.json
npm install

# Check Node version
node --version  # Should be 18+

# Clear Vite cache
rm -rf .vite
npm run dev
```

### Privy login fails
- Verify `VITE_PRIVY_APP_ID` is correct in `.env.development.local`
- Check Privy dashboard has login methods enabled
- Try incognito mode to avoid cache issues

### Smart account not created
- Verify `VITE_PIMLICO_API_KEY` is correct
- Check Mantle Sepolia is the selected network
- Check Pimlico API quota on dashboard

## Environment Variables Reference

### Backend (.env)
```env
# Server
PORT=10000
APP_ENV=development

# Database
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/seer

# Cache
REDIS_URL=redis://localhost:6379

# Network
MANTLE_RPC_URL=https://rpc.sepolia.mantle.xyz
MANTLE_CHAIN_ID=5003

# AI
CLAUDE_API_KEY=sk-...

# Data providers
NANSEN_API_KEY=nsn_...
```

### Frontend (.env.development.local)
```env
# Authentication
VITE_PRIVY_APP_ID=your_privy_app_id
VITE_PIMLICO_API_KEY=your_pimlico_api_key

# API
VITE_API_BASE=http://localhost:10000

# Network
VITE_CHAIN_ID=5003
VITE_RPC_URL=https://rpc.sepolia.mantle.xyz
```

## Next Steps

1. **Explore the Dashboard** — Navigate through Signal Feed, Agent, Arena, Identity
2. **Create an Intent** — Tell the agent what you want (e.g., "Buy 100 USDC worth of MNT")
3. **Make Predictions** — Bet on Seer's predictions in the Arena
4. **Build Your Identity** — Complete on-chain actions to build your reputation

## Architecture

```
┌─────────────────────────────────────────────────┐
│         Frontend (React + Vite)                 │
│  ├─ Privy Authentication                        │
│  └─ Pimlico Smart Account Creation              │
└─────────────────────────────────────────────────┘
                      ↓
         HTTP/REST API (Axum)
                      ↓
┌─────────────────────────────────────────────────┐
│         Backend (Rust)                          │
│  ├─ Authentication & Sessions                   │
│  ├─ Protocol Integration                        │
│  │  ├─ Agni Finance (Uniswap V3)               │
│  │  ├─ Merchant Moe (Liquidity Book)           │
│  │  ├─ mETH Protocol (Liquid Staking)          │
│  │  └─ Fluxion Network                          │
│  ├─ Signal Detection & ML                       │
│  ├─ Intent Parsing & Execution                  │
│  └─ Arena & Predictions                         │
└─────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────┐
│    Blockchain (Mantle Sepolia)                  │
│  ├─ Smart Accounts (ERC-4337)                   │
│  ├─ Protocol Contracts                          │
│  ├─ Arena Predictions                           │
│  └─ Identity SBT                                │
└─────────────────────────────────────────────────┘
```

## Support

- **Documentation** — See `README.md` files in each directory
- **API Docs** — See `PROTOCOL_INTEGRATION.md`
- **Issues** — Open an issue on GitHub
- **Discord** — Join the Mantle community Discord

---

**Happy hacking! 🚀**
