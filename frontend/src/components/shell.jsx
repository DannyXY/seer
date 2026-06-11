/* ============================================================
   SEER - app shell: sidebar + right context rail
   ============================================================ */
import { useState, useEffect } from 'react';

const NAV = [
  { id: "feed", label: "Signal Feed", icon: "signal" },
  { id: "agent", label: "My Agent", icon: "agent" },
  { id: "identity", label: "My Identity", icon: "identity" },
  { id: "arena", label: "The Arena", icon: "arena" },
  { id: "settings", label: "Settings", icon: "settings" },
];

function Sidebar({ route, setRoute, onDisconnect, badge, collapsed, onToggle }) {
  const seer = window.useSeerStore();
  const walletLabel = window.SEER.util.shortAddr(seer.wallet);
  return (
    <aside className={"seer-sidebar" + (collapsed ? " collapsed" : "")}>
      <div className="seer-side-top">
        <div className="seer-brand">
          <PrismMark size={collapsed ? 26 : 24} />
          {!collapsed && <span className="seer-wordmark">SEER</span>}
        </div>
        <button className="seer-collapse" onClick={onToggle} title={collapsed ? "Expand sidebar" : "Collapse sidebar"} aria-label="Toggle sidebar">
          <Icon name="chevR" size={15} style={{ transform: collapsed ? "none" : "rotate(180deg)", transition: "transform .2s var(--ease)" }} />
        </button>
      </div>

      <nav className="seer-nav">
        {NAV.map((n) => (
          <button key={n.id} className={"seer-nav-item" + (route === n.id ? " active" : "")} onClick={() => setRoute(n.id)} title={collapsed ? n.label : undefined}>
            <Icon name={n.icon} size={18} />
            {!collapsed && <span className="seer-nav-label">{n.label}</span>}
            {!collapsed && n.id === "feed" && badge > 0 && <span className="seer-nav-badge num">{badge}</span>}
          </button>
        ))}
      </nav>

      <div className="seer-side-foot">
        {!collapsed && (
          <div className="seer-net">
            <span className="dot live" style={{ background: "var(--c-opp)" }} />
            <span className="mono" style={{ fontSize: 11.5 }}>Mantle Network</span>
          </div>
        )}
        <div className="seer-wallet-chip" title={collapsed ? walletLabel : undefined}>
          <span className="center seer-wallet-av" />
          {!collapsed && <span className="mono grow" style={{ fontSize: 12.5 }}>{walletLabel}</span>}
          {!collapsed && <button className="btn-quiet" style={{ padding: 4, fontSize: 11 }} onClick={onDisconnect} title="Disconnect"><Icon name="ext" size={14} /></button>}
        </div>
      </div>
    </aside>
  );
}

function formatAssetAmount(asset) {
  const value = Number(asset.bal || 0);
  const symbol = String(asset.sym || "").toUpperCase();
  const decimals = ["MNT", "WMNT", "METH", "CMETH", "WETH"].includes(symbol) ? 4 : 2;
  return value.toLocaleString(undefined, { maximumFractionDigits: value >= 1000 ? 0 : decimals });
}

/* ---------- Right context rail ---------- */
function RightRail({ setRoute, riskScore, showToast }) {
  const { ASSETS, MAINNET_ASSETS, TESTNET_ASSETS, SEER_TOKEN_FAUCET, ACTIVE_INTENTS } = window.useSeerStore();
  const [balanceNetwork, setBalanceNetwork] = useState("mainnet");
  const [faucetBusy, setFaucetBusy] = useState(false);
  const mainnetAssets = MAINNET_ASSETS?.length ? MAINNET_ASSETS : (ASSETS || []);
  const testnetAssets = TESTNET_ASSETS || [];
  const visibleAssets = balanceNetwork === "testnet" ? testnetAssets : mainnetAssets;
  const totalUsd = visibleAssets.reduce((s, a) => s + Number(a.bal || 0) * Number(a.usd || 0), 0);
  const intent = ACTIVE_INTENTS.find((i) => i.status === "RUNNING");

  async function claimFaucet() {
    if (!SEER_TOKEN_FAUCET || faucetBusy) return;
    setFaucetBusy(true);
    try {
      const { txHash, faucet } = await window.SeerAPI.claimSeerTokens();
      showToast?.(`${faucet.amount} ${faucet.token_symbol} faucet submitted.`, 'success', txHash);
    } catch (err) {
      showToast?.(err.message || "Faucet claim failed.", 'error');
    } finally {
      setFaucetBusy(false);
    }
  }

  return (
    <aside className="seer-rail">
      <div className="seer-rail-block">
        <div className="row" style={{ justifyContent: "space-between", marginBottom: 14 }}>
          <span className="eyebrow">Wallet</span>
          <span className="num" style={{ fontSize: 12, color: "var(--ink-2)" }}>$<CountUp to={totalUsd} decimals={0} /></span>
        </div>
        <div className="seer-balance-tabs" role="tablist" aria-label="Wallet balance network">
          {["mainnet", "testnet"].map((network) => (
            <button
              key={network}
              className={"seer-balance-tab" + (balanceNetwork === network ? " active" : "")}
              role="tab"
              aria-selected={balanceNetwork === network}
              onClick={() => setBalanceNetwork(network)}
            >
              {network === "mainnet" ? "Mainnet" : "Testnet"}
            </button>
          ))}
        </div>
        <div className="col gap-12">
          {visibleAssets.length === 0 ? <div className="mut" style={{ fontSize: 13 }}>No {balanceNetwork} positions returned yet.</div> : visibleAssets.map((a) => (
            <div key={`${balanceNetwork}:${a.sym}`} className="row gap-12">
              <span className="center seer-asset-ic">{a.sym[0]}</span>
              <div className="col" style={{ lineHeight: 1.25 }}>
                <span style={{ fontSize: 13.5, fontWeight: 500 }}>{a.sym}</span>
                <span className="faint" style={{ fontSize: 11.5, whiteSpace: "nowrap" }}>{a.name}</span>
              </div>
              <div className="col grow" style={{ alignItems: "flex-end", lineHeight: 1.25 }}>
                <span className="num" style={{ fontSize: 13.5 }}>{formatAssetAmount(a)}</span>
              </div>
            </div>
          ))}
        </div>
        {balanceNetwork === "testnet" && (
          <button
            className="seer-faucet-link"
            disabled={!SEER_TOKEN_FAUCET || faucetBusy}
            onClick={claimFaucet}
            title={SEER_TOKEN_FAUCET ? "Claim Seer test USDC" : "Faucet is not configured for this environment"}
          >
            <span className="center seer-faucet-ic"><Icon name="bolt2" size={14} /></span>
            <span className="seer-faucet-copy">{faucetBusy ? "Opening wallet..." : "Seer token faucet"}</span>
            {SEER_TOKEN_FAUCET && <span className="seer-faucet-amt">+{SEER_TOKEN_FAUCET.amount} {SEER_TOKEN_FAUCET.token_symbol}</span>}
          </button>
        )}
      </div>

      <div className="seer-rail-block">
        <span className="eyebrow">Active agent</span>
        {intent ? (
          <div className="seer-agent-status" style={{ marginTop: 12 }}>
            <div className="row gap-8" style={{ marginBottom: 8 }}>
              <span className="dot live" /><span className="mono" style={{ fontSize: 11, color: "var(--c-opp)", letterSpacing: "0.06em" }}>RUNNING</span>
            </div>
            <div style={{ fontSize: 13, lineHeight: 1.4, marginBottom: 8 }}>{intent.summary}</div>
            <div className="faint" style={{ fontSize: 11.5 }}>Last action · {relTime(intent.lastTs)}</div>
            <div className="mut" style={{ fontSize: 12, marginTop: 2 }}>{intent.lastAction}</div>
          </div>
        ) : (
          <div className="mut" style={{ fontSize: 13, marginTop: 12 }}>No active agent - set an intent.</div>
        )}
      </div>

      <div className="seer-rail-block">
        <span className="eyebrow">Portfolio risk</span>
        <div className="center seer-risk-gauge"><RiskGauge score={riskScore} /></div>
        <div className="faint seer-risk-caption">Lower is safer · 0-100</div>
      </div>

      <button className="btn btn-primary" style={{ width: "100%", justifyContent: "center" }} onClick={() => setRoute("agent")}>
        New Intent<Icon name="arrow" size={16} />
      </button>
    </aside>
  );
}

export { Sidebar, RightRail, NAV };
