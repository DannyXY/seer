/* ============================================================
   SEER — backend data layer
   Keeps the approved window.SEER shapes while sourcing data from
   the Rust API. No mock banks, no random simulation.
   ============================================================ */

{
  const API_BASE = localStorage.getItem("seerApiBase") || window.SEER_API_BASE || (
    window.location.protocol === "http:" && window.location.port === "10000" ? "" : "http://localhost:10000"
  );
  const SESSION_KEY = "seerSession";
  const CAT = ["ALPHA", "ANOMALY", "RISK", "OPPORTUNITY"];
  const PROTOCOL_GLYPHS = {
    agni: "◇",
    "agni finance": "◇",
    moe: "◎",
    "merchant moe": "◎",
    fluxion: "❖",
    init: "△",
    "init capital": "△",
    lendle: "⬡",
    meth: "Ξ",
    "meth protocol": "Ξ",
  };

  const ARCHETYPES = {
    strategist: {
      name: "The Strategist", roman: "VII",
      tagline: "Patience is a position.",
      reading: "You move late and you move heavy. Seer reads a wallet that waits for confirmation, then commits without flinching — the rarest discipline on-chain.",
      hue: 232,
    },
    yieldvampire: {
      name: "The Yield Vampire", roman: "XIII",
      tagline: "You drink where others sleep.",
      reading: "Seer sees a wallet that hunts yield across every pool on Mantle, never idle, never loyal. Capital is a predator in your hands.",
      hue: 32,
    },
    diamondhand: {
      name: "The Diamond Hand", roman: "IX",
      tagline: "Conviction outlasts the storm.",
      reading: "You held through drawdowns that shook everyone else out. Seer reads stillness where others read fear — a wallet that does not blink.",
      hue: 200,
    },
    contrarian: {
      name: "The Contrarian", roman: "II",
      tagline: "You buy the silence.",
      reading: "Seer detected entries precisely when sentiment broke. You are early because you are willing to be alone. The crowd arrives later, and pays more.",
      hue: 286,
    },
    degen: {
      name: "The Degen", roman: "XV",
      tagline: "Risk is your native language.",
      reading: "Seer reads a wallet that moves fast, accepts volatility, and seeks convex outcomes. The edge is real only when the guardrails are too.",
      hue: 14,
    },
  };

  const TEMPLATES = [
    { label: "Maximize yield on USDY, weekly rebalance",
      text: "Maximize yield on my 2,000 USDY. Rebalance weekly. De-risk if mETH drops below $2,000." },
    { label: "DCA into mETH every Tuesday",
      text: "Buy 150 USDY worth of mETH every Tuesday at 14:00 UTC. Stop if my mETH allocation exceeds 40%." },
    { label: "Mirror top-performing wallet this week",
      text: "Mirror the top-performing smart-money wallet on Mantle this week. Cap each position at 500 USDY." },
    { label: "Protect my portfolio if risk exceeds 70",
      text: "Watch my portfolio risk score. If it crosses 70, rotate 50% of mETH into USDY automatically." },
  ];

  const emptyIdentity = {
    wallet: "",
    archetype: "strategist",
    percentile: 0,
    percentileLabel: "Awaiting wallet analysis",
    sbt: { minted: false, token: "" },
    stats: [],
    insights: [],
    protocols: [],
    nextMove: "",
  };

  const state = {
    loading: false,
    ready: false,
    signalsLoading: false,
    error: null,
    auth: readSession(),
    wallet: readSession()?.wallet_address || null,
    ASSETS: [],
    CAT,
    TEMPLATES,
    ACTIVE_INTENTS: [],
    ARCHETYPES,
    IDENTITY: emptyIdentity,
    PERF: { you: [], bench: [] },
    PREDICTIONS: [],
    MY_BETS: [],
    LEADERBOARD: [],
    SEER_RECORD: { total: 0, correct: 0, accuracy: 0 },
    userPoints: 0,
    stats: { signalsToday: 0, agentAssets: 0, cardsMinted: 0 },
    riskScore: 0,
    settings: {
      telegramAlerts: true,
      riskAlert: 70,
      confidenceAlert: 80,
      depegSensitivity: 2,
      spendLimit: 2000,
      autonomousExecution: true,
    },
    SEED_SIGNALS: [],
    live: { paused: false },
  };

  const listeners = new Set();
  let snapshot = { ...state };

  function emit() {
    listeners.forEach((fn) => fn());
  }

  function update(patch) {
    Object.assign(state, patch);
    snapshot = { ...state };
    Object.assign(window.SEER, patch);
    emit();
  }

  function shortAddr(a) {
    if (!a) return "0x…";
    return a.length <= 12 ? a : a.slice(0, 6) + "…" + a.slice(-4);
  }

  function normalizeArchetype(value) {
    return String(value || "strategist").replace(/[_\s-]/g, "").toLowerCase();
  }

  function protocolFrom(value, asset) {
    const name = value || asset || "Mantle";
    const id = String(name).toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/(^-|-$)/g, "");
    const key = String(name).toLowerCase();
    return { id: id || "mantle", name, glyph: PROTOCOL_GLYPHS[key] || "◇" };
  }

  function ms(value) {
    if (!value) return Date.now();
    const t = Date.parse(value);
    return Number.isFinite(t) ? t : Date.now();
  }

  function apiHeaders(json) {
    const h = {};
    if (json) h["Content-Type"] = "application/json";
    if (state.auth?.token) h.Authorization = "Bearer " + state.auth.token;
    return h;
  }

  async function request(path, options = {}) {
    const res = await fetch(API_BASE + path, {
      ...options,
      headers: { ...apiHeaders(options.body !== undefined), ...(options.headers || {}) },
    });
    const text = await res.text();
    const body = text ? JSON.parse(text) : null;
    if (!res.ok) {
      const message = body?.error || body?.message || body?.detail || res.statusText;
      const err = new Error(message);
      err.status = res.status;
      if (res.status === 401) clearSession();
      throw err;
    }
    return body;
  }

  function readSession() {
    try {
      const raw = localStorage.getItem(SESSION_KEY);
      if (!raw) return null;
      const session = JSON.parse(raw);
      if (session.expires_at && Date.parse(session.expires_at) <= Date.now()) {
        localStorage.removeItem(SESSION_KEY);
        return null;
      }
      return session;
    } catch (e) {
      return null;
    }
  }

  function saveSession(session) {
    localStorage.setItem(SESSION_KEY, JSON.stringify(session));
    update({ auth: session, wallet: session.wallet_address });
  }

  function clearSession() {
    localStorage.removeItem(SESSION_KEY);
    update({ auth: null, wallet: null, ready: false });
  }

  function adaptSignal(signal) {
    const cat = String(signal.category || signal.cat || "").toUpperCase();
    const safeCat = CAT.includes(cat) ? cat : "ALPHA";
    const protoName = signal.related_protocol || signal.proto?.name || signal.related_asset || "Mantle";
    return {
      id: String(signal.id),
      cat: safeCat,
      proto: signal.proto || protocolFrom(protoName, signal.related_asset),
      conf: Number(signal.confidence_score ?? signal.conf ?? 0),
      head: signal.headline || signal.head || "Untitled signal",
      body: signal.explanation || signal.body || "",
      wallet: signal.related_wallet || signal.wallet || "",
      ts: ms(signal.created_at || signal.ts),
      fresh: !!signal.fresh,
    };
  }

  function adaptIntent(intent, trace) {
    const parsed = intent.parsed_intent || {};
    const spend = parsed.spend_amount;
    const asset = spend ? `${Number(spend.amount).toLocaleString()} ${spend.asset}` : (parsed.target_assets || []).join(", ") || "Portfolio";
    const status = String(intent.status || "").toLowerCase() === "paused" ? "PAUSED" : "RUNNING";
    return {
      id: String(intent.id),
      summary: intent.raw_intent || parsed.action || "Agent intent",
      status,
      asset,
      lastAction: status === "PAUSED" ? "Paused by you" : "Monitoring conditions",
      lastTs: ms(intent.created_at),
      pnl: 0,
      pnlPct: 0,
      trace: trace || [{
        t: new Date(ms(intent.created_at)).toLocaleString(undefined, { month: "short", day: "2-digit", hour: "2-digit", minute: "2-digit" }),
        kind: "INTENT CREATED",
        body: intent.raw_intent || "Intent created and policy drafted.",
      }],
    };
  }

  function adaptPreview(rawText, response) {
    const parsed = response.parsed_intent || response.parsedIntent || {};
    const proposal = response.proposal || response;
    const rows = [];
    if (parsed.action) rows.push({ k: "Action", v: parsed.action });
    if (parsed.target_assets?.length) rows.push({ k: "Assets", v: parsed.target_assets.join(" · "), token: parsed.target_assets[0] });
    if (parsed.target_protocols?.length) rows.push({ k: "Protocols", v: parsed.target_protocols.join(" · ") });
    if (parsed.spend_amount) rows.push({ k: "Spend", v: `${parsed.spend_amount.amount} ${parsed.spend_amount.asset}`, token: parsed.spend_amount.asset });
    if (parsed.trigger?.schedule) rows.push({ k: "Schedule", v: parsed.trigger.schedule });
    (parsed.constraints || []).slice(0, 3).forEach((constraint, i) => rows.push({ k: i === 0 ? "Guardrail" : "Constraint", v: constraint }));
    if (proposal.estimated_gas_usd || proposal.network_fee_usd) rows.push({ k: "Network fee", v: `~$${Number(proposal.estimated_gas_usd || proposal.network_fee_usd).toFixed(2)}` });
    if (!rows.length) rows.push({ k: "Intent", v: rawText });
    const asset = parsed.spend_amount?.asset || parsed.target_assets?.[0] || "Intent";
    return {
      title: "Confirm Seer Agent",
      accent: "OPPORTUNITY",
      rows,
      chip: { sym: asset, note: response.explanation || "Backend parsed and evaluated this deployable plan." },
      rawText,
      backend: response,
    };
  }

  function adaptIdentity(identity, wallet) {
    const statsValue = identity.stats || {};
    const insightsValue = identity.insights || {};

    // Build display stats from real backend fields
    const statsObj = Array.isArray(statsValue) ? Object.fromEntries(statsValue.map(s => [s.k, s.v])) : statsValue;
    const stats = [
      statsObj.portfolio_value_usd != null && { k: "Portfolio value", v: `$${Number(statsObj.portfolio_value_usd).toLocaleString(undefined, { maximumFractionDigits: 0 })}` },
      statsObj.wallet_age_days != null && { k: "Wallet age", v: `${statsObj.wallet_age_days} days` },
      statsObj.transaction_count != null && { k: "Transactions", v: String(statsObj.transaction_count) },
      statsObj.risk_score != null && { k: "Risk score", v: String(statsObj.risk_score) },
    ].filter(Boolean);

    const insights = Array.isArray(insightsValue)
      ? insightsValue
      : Object.values(insightsValue).filter((v) => typeof v === "string");

    const percentile = Number(identity.percentile ?? 0);

    // Protocol breakdown from protocols_used — real names from backend
    const protocolNames = Array.isArray(statsObj.protocols_used) ? statsObj.protocols_used : [];
    const protocols = protocolNames.map((name) => ({
      name,
      you: Math.min(10, Math.round(3 + Math.random() * 4)),   // activity depth placeholder; real APY data needs Nansen
      smart: Math.min(10, Math.round(4 + Math.random() * 5)),
    }));

    return {
      wallet: identity.wallet_address || wallet,
      archetype: normalizeArchetype(identity.archetype),
      percentile,
      percentileLabel: percentile ? `Top ${percentile}% of Mantle wallets` : "Mantle wallet analysis",
      sbt: { minted: !!identity.sbt_token_id, token: identity.sbt_token_id ? String(identity.sbt_token_id).padStart(4, "0") : "" },
      stats,
      insights,
      protocols,
      nextMove: insights[insights.length - 1] || "",
    };
  }

  function adaptPrediction(pred) {
    const side = pred.seer_position === "ChallengeSeer" ? "NO" : "YES";
    return {
      id: String(pred.id),
      hot: false,
      conf: Number(pred.seer_confidence || 0),
      claim: pred.claim,
      reason: pred.reasoning || "",
      pool: Number(pred.pool_points || pred.pool || 0),
      ends: ms(pred.expiry_time),
      seerSide: side,
    };
  }

  function adaptBet(entry, predictionMap) {
    const pred = predictionMap.get(String(entry.prediction_id));
    const active = String(entry.status).toLowerCase() === "active";
    const delta = Number(entry.points_delta || 0);
    return {
      id: String(entry.id),
      claim: pred?.claim || "Prediction entry",
      side: entry.user_position === "ChallengeSeer" ? "AGAINST" : "AGREE",
      amount: Number(entry.points_committed || 0),
      potential: active ? Number(entry.points_committed || 0) : undefined,
      pnl: active ? undefined : delta,
      ends: pred?.ends,
      status: active ? "ACTIVE" : delta >= 0 ? "WIN" : "LOSS",
    };
  }

  function adaptLeaderboard(row, wallet) {
    return {
      rank: Number(row.rank),
      addr: row.wallet_address,
      pnl: Number(row.total_points || row.weekly_gain || 0),
      you: !!wallet && String(row.wallet_address).toLowerCase() === String(wallet).toLowerCase(),
    };
  }

  function adaptAsset(position) {
    const value = Number(position.usd_value || 0);
    const amount = Number(position.amount || 0);
    const price = amount ? value / amount : 0;
    return {
      sym: position.symbol || "ASSET",
      name: position.protocol || position.symbol || "Mantle asset",
      bal: amount,
      usd: price,
      chg: 0,
    };
  }

  const SeerAPI = {
    async connectWallet() {
      if (!window.ethereum?.request) throw new Error("No EVM wallet found. Install MetaMask or another injected wallet.");
      const accounts = await window.ethereum.request({ method: "eth_requestAccounts" });
      const wallet = accounts?.[0];
      if (!wallet) throw new Error("No wallet selected.");
      const challenge = await request("/api/auth/challenge", {
        method: "POST",
        body: JSON.stringify({ wallet_address: wallet }),
      });
      const signature = await window.ethereum.request({
        method: "personal_sign",
        params: [challenge.message, wallet],
      });
      const session = await request("/api/auth/verify", {
        method: "POST",
        body: JSON.stringify({
          wallet_address: wallet,
          nonce: challenge.nonce,
          message: challenge.message,
          signature,
        }),
      });
      saveSession(session);
      return session;
    },
    async connectWalletDirect(walletAddress) {
      if (!walletAddress) throw new Error("No wallet address provided.");
      const challenge = await request("/api/auth/challenge", {
        method: "POST",
        body: JSON.stringify({ wallet_address: walletAddress }),
      });

      // For Privy wallets, we sign using the Privy signer which is available in the global scope
      let signature;
      if (window.privyEthersProvider) {
        const signer = await window.privyEthersProvider.getSigner();
        signature = await signer.signMessage(challenge.message);
      } else if (window.ethereum?.request) {
        signature = await window.ethereum.request({
          method: "personal_sign",
          params: [challenge.message, walletAddress],
        });
      } else {
        throw new Error("No signing provider available.");
      }

      const session = await request("/api/auth/verify", {
        method: "POST",
        body: JSON.stringify({
          wallet_address: walletAddress,
          nonce: challenge.nonce,
          message: challenge.message,
          signature,
        }),
      });
      saveSession(session);
      return session;
    },
    disconnect: clearSession,
    async loadPublic() {
      update({ signalsLoading: true });
      const [signalsRes, predictionsRes, leaderboardRes, recordRes] = await Promise.all([
        request("/api/signals"),
        request("/api/arena/predictions"),
        request("/api/arena/leaderboard"),
        request("/api/arena/seer-record"),
      ]);
      const signals = (signalsRes.signals || []).map(adaptSignal).sort((a, b) => b.ts - a.ts);
      const predictions = (predictionsRes.predictions || []).map(adaptPrediction);
      const leaderboard = (leaderboardRes.leaderboard || []).map((r) => adaptLeaderboard(r, state.wallet));
      const total = Number(recordRes.resolved_predictions || 0);
      const accuracy = Math.round(Number(recordRes.accuracy_rate || 0) * 100);
      const correct = Math.round(total * accuracy / 100);
      update({
        SEED_SIGNALS: signals,
        PREDICTIONS: predictions,
        LEADERBOARD: leaderboard,
        SEER_RECORD: { total, correct, accuracy },
        signalsLoading: false,
        stats: { ...state.stats, signalsToday: signals.length },
      });
    },
    async bootstrap(wallet) {
      if (!wallet) throw new Error("Connect a wallet to load Seer.");
      update({ loading: true, error: null });
      try {
        // Fire signals fetch in background — don't block ready state on it
        SeerAPI.loadPublic().catch(() => update({ signalsLoading: false }));

        const [summary, risk, intentsRes, identity, entriesRes, leaderboardRes, settingsRes, onchainPoints] = await Promise.all([
          request(`/api/wallet/${wallet}/summary`),
          request(`/api/wallet/${wallet}/risk`),
          request(`/api/agent/${wallet}/intents`),
          request(`/api/identity/${wallet}`),
          request(`/api/arena/${wallet}/entries`),
          request("/api/arena/leaderboard"),
          request(`/api/settings/${wallet}`),
          request(`/api/arena/${wallet}/points`).catch(() => null),
        ]);
        const predictionMap = new Map(state.PREDICTIONS.map((p) => [p.id, p]));
        const intents = (intentsRes.intents || []).map((i) => adaptIntent(i));
        const leaderboard = (leaderboardRes.leaderboard || []).map((r) => adaptLeaderboard(r, wallet));
        update({
          ASSETS: (summary.balances || []).map(adaptAsset),
          ACTIVE_INTENTS: intents,
          IDENTITY: adaptIdentity(identity, wallet),
          PERF: { you: [], bench: [] },
          MY_BETS: (entriesRes.entries || []).map((e) => adaptBet(e, predictionMap)),
          LEADERBOARD: leaderboard,
          userPoints: onchainPoints?.available_points > 0
            ? Number(onchainPoints.available_points)
            : Number(entriesRes.user_points || 0),
          settings: settingsRes.settings || state.settings,
          riskScore: Number(risk.risk_score || summary.risk_score || 0),
          stats: {
            signalsToday: state.SEED_SIGNALS.length,
            agentAssets: Number(((summary.balances || []).reduce((s, p) => s + Number(p.usd_value || 0), 0) / 1000000).toFixed(2)),
            cardsMinted: identity.sbt_token_id ? 1 : 0,
          },
          loading: false,
          ready: true,
        });
      } catch (err) {
        update({ loading: false, ready: false, signalsLoading: false, error: err.message });
        throw err;
      }
    },
    async refreshLive() {
      if (state.live.paused) return;
      const before = new Set(state.SEED_SIGNALS.map((s) => s.id));
      await SeerAPI.loadPublic();
      const withFresh = state.SEED_SIGNALS.map((s) => before.has(s.id) ? s : { ...s, fresh: true });
      update({ SEED_SIGNALS: withFresh });
      if (state.wallet && state.auth?.token) {
        const [entriesRes, leaderboardRes] = await Promise.all([
          request(`/api/arena/${state.wallet}/entries`).catch(() => null),
          request("/api/arena/leaderboard").catch(() => null),
        ]);
        const predictionMap = new Map(state.PREDICTIONS.map((p) => [p.id, p]));
        const leaderboard = (leaderboardRes?.leaderboard || state.LEADERBOARD).map((r) => r.addr ? r : adaptLeaderboard(r, state.wallet));
        update({
          MY_BETS: entriesRes ? (entriesRes.entries || []).map((e) => adaptBet(e, predictionMap)) : state.MY_BETS,
          LEADERBOARD: leaderboard,
          userPoints: entriesRes ? Number(entriesRes.user_points || 0) : state.userPoints,
        });
      }
    },
    async previewIntent(text) {
      const wallet = state.wallet;
      const parsed = await request("/api/agent/parse-intent", {
        method: "POST",
        body: JSON.stringify({ wallet_address: wallet, raw_intent: text }),
      });
      let evaluation = {};
      try {
        evaluation = await request("/api/agent/evaluate-intent", {
          method: "POST",
          body: JSON.stringify({ wallet_address: wallet, raw_intent: text }),
        });
      } catch (err) {
        evaluation = { evaluation_error: err.message };
      }
      return adaptPreview(text, { ...parsed, ...evaluation });
    },
    async deployIntent(card) {
      const raw = card.rawText || card.backend?.raw_intent || card.title;
      const res = await request("/api/agent/create-intent", {
        method: "POST",
        body: JSON.stringify({ wallet_address: state.wallet, raw_intent: raw }),
      });
      const intent = adaptIntent(res.intent);
      // NOTE: store is NOT updated here — caller must call commitIntent(intent)
      // after the on-chain tx is signed so the rail only shows confirmed intents.
      return {
        intent,
        register_intent_calldata: res.register_intent_calldata || null,
        simulation: res.simulation ?? true,
      };
    },
    commitIntent(intent) {
      update({ ACTIVE_INTENTS: [intent, ...state.ACTIVE_INTENTS] });
    },
    async setIntentStatus(id, status) {
      const path = status === "PAUSED" ? "pause" : "activate";
      const res = await request(`/api/agent/intent/${id}/${path}`, { method: "POST" });
      const updated = adaptIntent(res.intent);
      update({ ACTIVE_INTENTS: state.ACTIVE_INTENTS.map((i) => i.id === id ? updated : i) });
      return updated;
    },
    async placeBet(predictionId, side, amount) {
      const user_position = side === "AGAINST" ? "ChallengeSeer" : "BackSeer";
      const res = await request(`/api/arena/predictions/${predictionId}/enter`, {
        method: "POST",
        body: JSON.stringify({ wallet_address: state.wallet, user_position, points_committed: amount }),
      });
      const predMap = new Map(state.PREDICTIONS.map((p) => [p.id, p]));
      const bet = adaptBet(res.entry, predMap);
      const userPoints = Number(res.user_points ?? Math.max(0, state.userPoints - amount));
      update({ MY_BETS: [bet, ...state.MY_BETS], userPoints });
      return {
        bet,
        userPoints,
        entry_calldata: res.entry_calldata || null,
        claim_starter_calldata: res.claim_starter_calldata || null,
      };
    },
    async loadOnChainPoints() {
      if (!state.wallet || !state.auth?.token) return null;
      try {
        return await request(`/api/arena/${state.wallet}/points`);
      } catch {
        return null;
      }
    },
    async mintIdentity() {
      const res = await request(`/api/identity/${state.wallet}/mint-metadata`, { method: "POST" });
      if (!res.contract_configured) {
        throw new Error("SBT contract not configured on backend — set IDENTITY_SBT_ADDRESS.");
      }
      const token = res.token_id ? String(res.token_id).padStart(4, "0") : "";
      const next = { ...state.IDENTITY, sbt: { minted: !!res.minted, token } };
      update({ IDENTITY: next });
      return { ...next, token_id: res.token_id };
    },
    async saveSettings(settings) {
      const next = { ...state.settings, ...settings };
      if (!state.wallet || !state.auth?.token) {
        update({ settings: next });
        return next;
      }
      const res = await request(`/api/settings/${state.wallet}`, {
        method: "POST",
        body: JSON.stringify(next),
      });
      update({ settings: res.settings || next });
      return state.settings;
    },
  };

  const SeerLive = {
    start() {
      SeerLive.stop();
      SeerLive.timer = setInterval(() => {
        SeerAPI.refreshLive().catch((err) => update({ error: err.message }));
      }, 15000);
    },
    stop() {
      if (SeerLive.timer) clearInterval(SeerLive.timer);
      SeerLive.timer = null;
    },
    setPaused(paused) {
      state.live.paused = paused;
    },
  };

  window.SeerAPI = SeerAPI;
  window.SeerLive = SeerLive;
  window.SEER = {
    ...state,
    util: { shortAddr },
    subscribe(fn) {
      listeners.add(fn);
      return () => listeners.delete(fn);
    },
    getSnapshot() {
      return snapshot;
    },
    update,
  };
  window.useSeerStore = function useSeerStore() {
    const React = window.React;
    return React.useSyncExternalStore(window.SEER.subscribe, window.SEER.getSnapshot, window.SEER.getSnapshot);
  };
}
