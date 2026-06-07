# Seer Frontend

React-based web interface for the Seer protocol agent on Mantle.

## Directory Structure

```
frontend/
├── src/
│   ├── app.jsx              # Root app component
│   ├── components/          # Reusable UI components
│   │   ├── shell.jsx        # Layout wrapper
│   │   ├── cardrender.jsx   # Card rendering
│   │   ├── signalfeed.jsx   # Signal feed display
│   │   ├── settings.jsx     # Settings panel
│   │   ├── tweaks.jsx       # Tweaks panel
│   │   ├── tweaks-panel.jsx # Tweaks UI
│   │   └── primitives.jsx   # Basic primitives
│   ├── pages/               # Page components
│   │   ├── landing.jsx      # Landing page
│   │   ├── agent.jsx        # Agent execution page
│   │   ├── agentscreen.jsx  # Agent screen
│   │   ├── arena.jsx        # Arena/predictions
│   │   └── identity.jsx     # Identity/SBT
│   ├── styles/              # Global & page styles
│   │   ├── styles.css       # Global styles
│   │   ├── dashboard.css    # Dashboard styles
│   │   └── screens.css      # Responsive styles
│   └── utils/               # Utilities
│       └── data.jsx         # Data helpers
├── public/                  # Static assets
│   └── index.html          # HTML entry point
├── package.json            # Dependencies & scripts
├── vite.config.js         # Vite configuration (optional)
└── README.md              # This file
```

## Setup

### Prerequisites

1. **Privy Account** — Create at [privy.io](https://privy.io)
   - Create a new application
   - Copy your App ID
   
2. **Pimlico API Key** — Get from [pimlico.io](https://pimlico.io)
   - Create an account and project
   - Copy your API key for Mantle Sepolia

### Install Dependencies

```bash
cd frontend
npm install
```

### Configure Environment

Create `.env.development.local` with your Privy and Pimlico credentials:

```bash
cp .env.example .env.development.local
```

Then edit `.env.development.local`:

```env
VITE_PRIVY_APP_ID=your_privy_app_id_here
VITE_PIMLICO_API_KEY=your_pimlico_api_key_here
VITE_API_BASE=http://localhost:10000
```

### Start Frontend Development Server

```bash
npm run dev
# Frontend runs on http://localhost:5173
```

### Build for Production

```bash
npm run build
# Creates optimized build in dist/
```

## Architecture

The frontend has been migrated from a CDN-based React setup to **Vite** with ES modules:

- **Build Tool**: Vite for fast development and optimized production builds
- **Auth**: Privy for social logins (email, Google, GitHub, Twitter, Discord)
- **Smart Accounts**: Pimlico for account abstraction on Mantle Sepolia
- **State Management**: Custom store pattern (window.SEER) with React external store API

## Features

### Pages

- **Landing** (`pages/landing.jsx`) — Welcome & onboarding
- **Agent** (`pages/agent.jsx`) — Intent creation & execution
- **AgentScreen** (`pages/agentscreen.jsx`) — Agent dashboard
- **Arena** (`pages/arena.jsx`) — Prediction markets
- **Identity** (`pages/identity.jsx`) — User identity & SBT

### Components

- **Shell** — Main layout wrapper (sidebar, header, footer)
- **CardRender** — Generic card component for data display
- **SignalFeed** — Live signal/alert feed
- **Settings** — User preferences
- **Tweaks** — Debug/feature toggles panel
- **Primitives** — Basic UI elements (buttons, inputs, etc.)

### Styles

- **Responsive Design** — Mobile-first approach
- **Dark Mode Support** — CSS custom properties for theming
- **Component Scoping** — CSS modules where needed

## Integration with Backend

The frontend communicates with the backend API at `http://localhost:10000`:

**Key Endpoints Used:**

- `POST /api/agent/parse-intent` — Parse user intent
- `POST /api/agent/evaluate-intent` — Evaluate & get proposal
- `POST /api/contracts/send-user-operation` — Submit execution
- `GET /api/positions/:address` — Get user's LP positions
- `POST /api/positions/agni` — Record Agni position
- `POST /api/arena/predictions` — Create prediction

## Development

### Adding a New Page

1. Create `src/pages/YourPage.jsx`
2. Add styles to `src/pages/YourPage.css`
3. Import & add route in `src/app.jsx`

Example:

```jsx
// src/pages/yourpage.jsx
export default function YourPage() {
  return (
    <div className="your-page">
      <h1>Your Page</h1>
    </div>
  );
}
```

### Adding a New Component

1. Create `src/components/YourComponent.jsx`
2. Keep styles in the component file or use `src/styles/`
3. Import in pages/components that need it

Example:

```jsx
// src/components/YourComponent.jsx
export default function YourComponent({ title, children }) {
  return (
    <div className="your-component">
      <h2>{title}</h2>
      {children}
    </div>
  );
}
```

### Styling Guidelines

- Use CSS custom properties for colors/spacing
- Mobile-first media queries
- BEM naming convention for class names
- Keep component styles scoped where possible

```css
/* Example */
.your-component {
  padding: var(--spacing-md);
  color: var(--color-text);
  background: var(--color-bg);
}

.your-component__title {
  font-size: var(--font-size-lg);
  margin-bottom: var(--spacing-sm);
}

@media (max-width: 768px) {
  .your-component {
    padding: var(--spacing-sm);
  }
}
```

## Environment Variables

Frontend environment variables should be set in `.env.development.local`:

```env
# Privy authentication (get from https://privy.io)
VITE_PRIVY_APP_ID=your_app_id_here

# Pimlico account abstraction (get from https://pimlico.io)
VITE_PIMLICO_API_KEY=your_api_key_here

# Backend API URL
VITE_API_BASE=http://localhost:10000

# Network configuration
VITE_CHAIN_ID=5003
VITE_RPC_URL=https://rpc.sepolia.mantle.xyz
```

## Dependencies

- **React 18** — UI framework
- **Vite** — Build tool & dev server
- **Privy** — Social authentication & embedded wallets
- **Pimlico** — Smart account abstraction (ERC-4337)
- **ethers.js** — Blockchain utilities
- **viem** — Ethereum/Mantle type-safe utilities

## Build & Deployment

### Development

```bash
npm run dev
# Runs on http://localhost:5173
```

### Production

```bash
npm run build
# Creates optimized build in dist/

npm run preview
# Test production build locally
```

## Authentication Flow

### Privy-Powered Login

1. User clicks "Sign in with Privy" on landing page
2. Privy modal shows social & wallet options
3. On successful auth:
   - User's wallet address is captured
   - Smart account is created via Pimlico
   - Backend session is established
4. User is redirected to app dashboard

### Smart Account Creation

- **When**: Automatically on first login
- **Who**: Pimlico bundler
- **How**: Deploys ERC-4337 compatible account on Mantle
- **Cost**: Sponsored via Pimlico paymaster (free for testing)

## Troubleshooting

**Privy login not working:**
- Verify `VITE_PRIVY_APP_ID` is set correctly
- Check Privy dashboard for enabled login methods
- Check browser console for error messages

**Smart account creation fails:**
- Verify `VITE_PIMLICO_API_KEY` is valid
- Check network is set to Mantle Sepolia
- Check Pimlico dashboard for API quota

**Backend API not connecting:**
- Verify `VITE_API_BASE=http://localhost:10000`
- Check backend is running (`npm run dev` in backend/)
- Check CORS is enabled in backend

**Styling issues:**
- Clear browser cache
- Check CSS custom properties are defined
- Verify class names match CSS

**Build errors:**
- Delete `node_modules` and `package-lock.json`
- Run `npm install` again
- Clear Vite cache: `rm -rf .vite`

## Completed Features

✅ Privy social authentication
✅ Smart account creation on login
✅ Vite build system with ES modules
✅ Multi-page routing
✅ Signal feed & data integration
✅ Agent intent management
✅ Arena predictions
✅ User identity & SBT tracking

## Next Steps

1. **Test End-to-End** — Sign in with Privy, verify smart account is created
2. **Complete UI Screens** — Finish remaining dashboard screens
3. **Real Data Loading** — Connect to backend API endpoints for live data
4. **Error Handling** — Add toast notifications & error boundaries
5. **Testing** — Add Vitest & React Testing Library tests
6. **Deployment** — Deploy frontend to Vercel or similar CDN

---

For backend API documentation, see `PROTOCOL_INTEGRATION.md` in the root directory.
