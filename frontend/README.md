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

### Start Frontend Server

```python
# Run the development server (serves on port 8088)
python3 serve.py

# Frontend will be available at http://localhost:8088/
```

The `serve.py` script serves the frontend as a single-page application (SPA) from the `public/` directory. All routes that don't match a static file will serve `public/index.html`.

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

Frontend environment variables should be set in a `.env` file:

```env
# Backend API URL
VITE_API_URL=http://localhost:10000

# Network configuration
VITE_CHAIN_ID=5003
VITE_RPC_URL=https://rpc.sepolia.mantle.xyz
```

## Dependencies

- **React 18** — UI framework
- **Vite** — Build tool & dev server
- **Axios** — HTTP client (for API calls)
- **viem** — Ethereum/Mantle utilities
- **wagmi** — React hooks for Ethereum

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

### Docker Deployment

```dockerfile
# Build stage
FROM node:18 AS builder
WORKDIR /app
COPY package*.json ./
RUN npm install
COPY . .
RUN npm run build

# Runtime stage
FROM node:18
WORKDIR /app
RUN npm install -g serve
COPY --from=builder /app/dist ./dist
EXPOSE 3000
CMD ["serve", "-s", "dist", "-l", "3000"]
```

## Troubleshooting

**Backend API not connecting:**
- Verify `VITE_API_URL` environment variable
- Check backend is running on port 10000
- Check CORS is enabled in backend (it is by default)

**Styling issues:**
- Clear browser cache
- Check CSS custom properties are defined
- Verify class names match CSS

**Build errors:**
- Delete `node_modules` and `package-lock.json`
- Run `npm install` again
- Clear Vite cache: `rm -rf .vite`

## Next Steps

1. **Wallet Integration** — Add Web3 wallet connection (MetaMask, WalletConnect)
2. **Real Data Loading** — Connect to backend API endpoints
3. **Error Handling** — Add toast notifications & error boundaries
4. **State Management** — Consider Redux or Zustand for complex state
5. **Testing** — Add Vitest & React Testing Library tests

---

For backend API documentation, see `PROTOCOL_INTEGRATION.md` in the root directory.
