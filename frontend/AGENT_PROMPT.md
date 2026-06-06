# Seer — Backend Integration Brief (for a coding agent)

You are wiring an existing, finished React frontend to a real backend. The UI is **done and approved** — your job is **data, auth, and actions**, not redesign. Do not restyle, re-layout, or “improve” the UI. Treat the visual system as frozen.

This brief tells you exactly how the app is built, where the single data seam is, and the contract every screen expects. Read it fully before writing code.

---

## 1. What this is

Seer is an AI-native on-chain intelligence + execution dashboard for the **Mantle** network. Five surfaces:

- **Signal Feed** — live, confidence-scored on-chain intelligence cards.
- **My Agent** — a conversational agent; user states an intent in natural language, confirms, and the agent deploys/executes on-chain. Active “intents” run in the background with an audit trace.
- **The Arena** — Seer publishes predictions; users bet **points** (not money) for/against, with a leaderboard and Seer’s track record.
- **My Identity** — a wallet “archetype” + percentile + stats + a soulbound identity card.
- **Settings** — guardrails, spending caps, risk threshold.

There is also a **Landing** page (pre-connect) and a wallet **connect** flow.

---

## 2. Stack & architecture (read carefully — this is unusual)

- **No build step, no npm, no bundler.** The app is plain files loaded by `seer/Seer.html`.
- React 18 + ReactDOM + Babel Standalone are loaded from `unpkg` via pinned `<script>` tags. All app code is **inline JSX** in `.jsx` files loaded as `<script type="text/babel">`.
- **Components do NOT use ES modules.** They share scope by attaching to `window` (e.g. `window.Landing`, `window.Icon`, `window.SEER`). Keep this pattern — do not convert to `import`/`export` or introduce a bundler unless you also rewrite `Seer.html` and the whole load order. Prefer **not** to.
- CSS is plain files linked from `Seer.html` (`styles.css`, `landing.css`, `dashboard.css`, `screens.css`, `agent.css`, `arena.css`, `identity.css`). A design-token layer lives in `styles.css` (CSS custom properties). **Do not touch tokens or class names.**
- File map:
  - `Seer.html` — entry + script/style load order.
  - `data.jsx` — **THE DATA SEAM.** An IIFE that builds the global `window.SEER` object and runs a fake “live” simulation. **This is the file you replace.**
  - `app.jsx` — root component, routing between landing/app, connect state, toasts.
  - `shell.jsx` — sidebar + layout.
  - `signalfeed.jsx`, `agentscreen.jsx`, `agent.jsx`, `arena.jsx`, `identity.jsx`, `settings.jsx`, `cardrender.jsx` — screens.
  - `primitives.jsx` — shared UI (Icon, CountUp, badges, orb, etc.).
  - `tweaks*.jsx` — a dev-only tweak panel; ignore/strip for production.

---

## 3. The one seam you cut: `window.SEER`

**Every screen reads from the global `window.SEER`.** It is built synchronously in `data.jsx` today, entirely from mock banks and `Math.random()`, including a `setInterval`-driven simulation that fabricates new signals and ticks numbers so the demo “feels live.”

Your task: **replace all mock generation with real data from my backend, preserving the exact object shapes below.** Do not leave any field served by random/mock fallback. If the backend can’t supply something, render an explicit empty/loading/error state — never silently fall back to fake defaults.

### Critical change: sync → async

`data.jsx` currently runs synchronously *before* React mounts. Real data is async. Restructure so the app:

1. Mounts immediately into a **loading state** (the connect/landing screen needs no auth; the dashboard does).
2. After wallet connect/auth, **fetches the initial payloads**, populates a data store, and renders.
3. Subscribes to **live updates** (WebSocket preferred; SSE or polling acceptable) for the feed, tickers, intent traces, arena pools, and points balance — these are the things that currently fake-tick.

Recommended approach (keep it simple, no new framework):
- Replace the `data.jsx` IIFE with a small **data layer**: `window.SeerAPI` (REST calls) + `window.SeerLive` (socket subscription) + a `window.SEER` store that components read, plus a lightweight pub/sub so screens re-render on push. A React context or a tiny `useSyncExternalStore` wrapper is ideal. **Do not change the components’ read sites more than necessary** — if you keep the `window.SEER.<X>` shape and add a subscribe hook, most screens need only a one-line change to re-render on updates.
- Keep `window.SEER.util` (`shortAddr`, etc.) — it’s pure formatting, reuse it.

---

## 4. Data contract (shapes the UI already expects)

Match these exactly. Field names, types, and enums are load-bearing — the components destructure them directly. Endpoints are **suggested**; adapt to my actual backend (see §6) but keep the response shapes.

### 4.1 Auth / wallet
- Connect is currently faked in the landing + a connect modal. Wire to real wallet connection (the project targets Mantle / EVM — use whatever my backend expects: SIWE-style signature, session token, etc.).
- On connect you must resolve the **session wallet address** and gate all dashboard data behind it.

### 4.2 Signals — `window.SEER` feed
Each signal:
```
{
  id: string,
  cat: "ALPHA" | "ANOMALY" | "RISK" | "OPPORTUNITY",   // drives color + badge — enum is fixed
  proto: { id: string, name: string, glyph: string },   // protocol; glyph is a short symbol char
  conf: number,            // 0–100 confidence, integer
  head: string,            // headline
  body: string,            // 1–3 sentence explanation WITH evidence (wallets/timing/why)
  wallet: string,          // 0x… address the signal references
  ts: number,              // epoch ms
  fresh: boolean           // true => arrival animation; set true for pushed items
}
```
- Initial load: a reverse-chronological page of recent signals.
- **Live:** push new signals over the socket with `fresh: true`. The feed prepends them with an arrival animation and a “new” badge. The header shows a “signals today” count — serve it, don’t compute client-side from the random simulator.
- `cat` MUST be one of the four enums; the spectrum/colors and filters key off it.

### 4.3 Agent — intents
Active intents:
```
{
  id: string,
  summary: string,         // one-line description
  status: "RUNNING" | "PAUSED",
  asset: string,           // e.g. "2,000 USDY" — display string
  lastAction: string,
  lastTs: number,          // epoch ms
  pnl: number,             // absolute, display currency-agnostic
  pnlPct: number,
  trace: [ { t: string, kind: string, body: string } ]   // audit log, newest first; t is a preformatted timestamp string
}
```
Natural-language → intent: the agent screen takes a free-text intent, then renders a **confirm card** (structured preview: actions, guardrails, caps) before deploying. Wire these actions to the backend:
- **Parse/preview:** POST the user’s text → backend returns the structured confirm-card payload (the deployable plan). Do not parse intent client-side.
- **Deploy:** POST confirm → backend arms/executes; returns the new intent object; prepend to the list.
- **Cancel:** the confirm card has a Cancel that must discard the plan with **no** backend mutation (other than maybe an analytics ping).
- **Pause/Resume:** toggle on an intent → PATCH status; reflect returned object.
- The “templates” (starter suggestions) can stay as static UI affordances, or come from the backend — your call, but the *result* of choosing one must round-trip through the real parse/preview endpoint.

### 4.4 Identity
```
ARCHETYPES: { [key]: { name, roman, tagline, reading, hue } }   // catalog; can stay static or come from backend
IDENTITY: {
  wallet: string,
  archetype: string,             // key into ARCHETYPES
  percentile: number,
  percentileLabel: string,
  sbt: { minted: boolean, token: string },
  stats: [ { k: string, v: string|number } ],
  insights: string[],
  protocols: [ { name: string, you: number, smart: number } ],  // APY you vs smart-money
  nextMove: string
}
PERF: { you: number[], bench: number[] }   // chart series, equal length
```
- `IDENTITY` is per-connected-wallet — fetch on connect.
- The identity **card** (`cardrender.jsx`) renders from `IDENTITY` + `ARCHETYPES[archetype]`. Minting the SBT should call the backend / chain and flip `sbt.minted` + set `sbt.token` from the real tx.

### 4.5 Arena — predictions, bets, points
```
PREDICTIONS: [ {
  id, hot: boolean, conf: number,
  claim: string, reason: string,
  pool: number,            // PRIZE POOL IN POINTS (not $)
  ends: number,            // epoch ms deadline
  seerSide: "YES" | "NO"
} ]
MY_BETS: [ {
  id, claim, side: "AGREE" | "AGAINST",
  amount: number,          // POINTS staked
  potential?: number,      // points (ACTIVE bets)
  pnl?: number,            // points (settled bets, +/-)
  ends?: number,
  status: "ACTIVE" | "WIN" | "LOSS"
} ]
LEADERBOARD: [ { rank: number, addr: string, pnl: number, you: boolean } ]   // pnl in POINTS
SEER_RECORD: { total: number, correct: number, accuracy: number }            // accuracy 0–100
userPoints: number         // the connected user's POINTS balance — shown in the Arena header, deducts on bet
```
- **Everything in the Arena is a POINTS system. There is no money.** Keep it that way.
- **Place bet:** POST `{ predictionId, side, amount }`. Backend validates against the user’s `userPoints`, deducts, returns the new balance + the created bet. The UI already: deducts the header balance, prepends to My Bets, and **disables the place button when stake > balance** — keep that guard but make the balance authoritative from the backend response.
- **Live:** pools (`pool`) and `userPoints` should update over the socket; they currently fake-tick.

### 4.6 Misc header stats
```
stats: { signalsToday: number, agentAssets: number, cardsMinted: number }
```
Used on landing/headers. Serve real values.

---

## 5. Behaviors to wire (currently mocked)

| Action | Current (mock) | Wire to |
|---|---|---|
| Connect wallet | fake modal → sets connected=true | real wallet auth/session |
| Feed updates | `setInterval` fabricates signals | socket subscription |
| Tickers / counts | `Math.random()` drift | backend values / socket |
| Deploy agent intent | local push | parse→confirm→deploy endpoints |
| Cancel confirm | local remove | discard, no mutation |
| Pause/resume intent | local status flip | PATCH status |
| Mirror a signal | toast only | execute/mirror endpoint (non-custodial) |
| Place bet | local deduct + push | POST bet, authoritative balance |
| Mint identity SBT | local flag flip | chain/backend mint tx |
| Settings (caps/risk/guardrails) | local state | persist to backend |

---

## 6. API surface (propose, then confirm with me)

I have a backend; I will give you the base URL, auth scheme, and socket protocol. **Before coding, produce a short proposed endpoint list mapping each shape above to a route**, e.g.:

```
POST /auth/siwe            -> session
GET  /me                   -> { wallet, userPoints, stats }
GET  /signals?cursor=      -> Signal[]            (paged)
WS   /signals/stream       -> push Signal (fresh:true)
GET  /intents              -> Intent[]
POST /agent/preview        -> ConfirmCard         (from { text })
POST /agent/deploy         -> Intent
PATCH /intents/:id         -> Intent              ({ status })
GET  /identity/:wallet     -> { IDENTITY, PERF }
GET  /arena                -> { PREDICTIONS, SEER_RECORD, LEADERBOARD }
GET  /arena/my-bets        -> Bet[]
POST /arena/bet            -> { bet, userPoints }
WS   /arena/stream         -> push { pool updates, userPoints }
POST /settings            -> persisted settings
```
Adapt names to my real backend — I’ll correct you. **Do not invent data; if a field has no source, ask.**

---

## 7. Hard constraints

1. **Do not restyle or re-layout.** No new components, colors, fonts, spacing, or class renames. The design is frozen. If a backend field is longer/shorter than mock copy, handle overflow gracefully within existing styles — don’t redesign.
2. **No mock fallbacks in production.** Remove `Math.random()` data, the fake signal simulator, and all hardcoded banks (`SIGNAL_BANK`, `ACTIVE_INTENTS`, `PREDICTIONS`, `LEADERBOARD`, `IDENTITY`, etc.). Replace with backend reads. Keep `util` formatters.
3. **Preserve the `window.SEER` shapes** (or introduce a typed store with identical shapes) so the screens keep working with minimal edits.
4. **Keep the no-bundler, `window`-global, `text/babel` architecture** unless you fully migrate `Seer.html` and prove it still runs by opening the file directly. If you do migrate to a real build (Vite, etc.), that’s acceptable **only** if the rendered UI is pixel-identical and all script load order is preserved — otherwise keep it as-is.
5. **Enums are fixed:** signal `cat`, intent `status`, bet `side`/`status`, `seerSide`. Color, filtering, and badges key off them.
6. **Money stays as points in the Arena.** Never reintroduce currency there.
7. **Security:** all execution actions (deploy/mirror/bet/mint) must be authorized server-side against the session wallet; never trust client balances. The UI’s optimistic updates must reconcile to backend responses.
8. **HTML must stay canonical** (explicit closing tags, double-quoted attributes) — the file supports direct visual editing; don’t emit self-closing non-void tags or minify the HTML.

---

## 8. Suggested execution order

1. Read `Seer.html` load order + `data.jsx` end-to-end; map every `window.SEER.*` read across the `.jsx` files (grep for `window.SEER`).
2. Propose the endpoint mapping (§6) and confirm with me.
3. Build the async data layer (`SeerAPI` + `SeerLive` + store + subscribe hook); mount app into loading state.
4. Wire auth/connect; gate dashboard fetches.
5. Port screens one at a time to the store: Feed → Agent → Arena → Identity → Settings. Verify each against the shapes in §4.
6. Replace the fake live simulation with real socket pushes.
7. Delete all mock banks; confirm no `Math.random()` data paths remain.
8. Smoke-test the full flow: connect → feed streams → deploy an intent → place a bet (balance reconciles) → mint identity.

Ask me for the backend base URL, auth scheme, and socket contract before step 3.
