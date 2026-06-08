/* ============================================================
   SEER — Signal Feed (hero screen)
   ============================================================ */
import { useState, useEffect, useRef } from 'react';

function SignalCard({ s, isNew, onMirror, onDismiss }) {
  const high = s.conf >= 80;
  const catColor = CAT_STYLE[s.cat].c;
  return (
    <article className={"seer-signal" + (isNew ? " arrive" : "") + (high ? " hi" : "")}>
      <div className="seer-signal-body">
        <div className="row gap-8" style={{ marginBottom: 12, flexWrap: "wrap" }}>
          <CategoryBadge cat={s.cat} />
          <ProtocolBadge proto={s.proto} />
          {high && <span className="badge" style={{ color: "var(--volt)", borderColor: "var(--coral-line)", background: "var(--coral-wash)" }}><Icon name="spark" size={11} />High conviction</span>}
          <span className="grow" />
          <span className="mono faint" style={{ fontSize: 11.5 }}>{relTime(s.ts)}</span>
        </div>

        <h3 className="seer-signal-head">{s.head}</h3>
        <p className="seer-signal-text">{s.body}</p>

        <div className="row gap-16" style={{ marginTop: 18, flexWrap: "wrap" }}>
          <div className="col" style={{ width: 188, gap: 5 }}>
            <span className="eyebrow" style={{ fontSize: 10 }}>Confidence</span>
            <ConfidenceBar value={s.conf} color="var(--volt)" />
          </div>
          <span className="grow" />
          <a className="seer-wallet-link mono" title="View on explorer">
            {window.SEER.util.shortAddr(s.wallet)}<Icon name="ext" size={12} />
          </a>
          <div className="row gap-8">
            <button className="btn btn-ghost" style={{ padding: "8px 12px" }} onClick={() => onDismiss(s.id)}>Dismiss</button>
            <button className="btn btn-primary" style={{ padding: "8px 14px" }} onClick={() => onMirror(s)}>Mirror this<Icon name="arrow" size={15} /></button>
          </div>
        </div>
      </div>
    </article>
  );
}

const FILTERS = ["ALL", "ALPHA", "ANOMALY", "RISK", "OPPORTUNITY"];

export function SignalFeed({ onMirror }) {
  const seer = window.useSeerStore();
  const [dismissed, setDismissed] = useState(new Set());
  const [filter, setFilter] = useState("ALL");
  const [query, setQuery] = useState("");
  const [newIds, setNewIds] = useState(new Set());
  const [paused, setPaused] = useState(false);

  useEffect(() => { window.SeerLive.setPaused(paused); }, [paused]);
  useEffect(() => {
    const fresh = seer.SEED_SIGNALS.filter((s) => s.fresh).map((s) => s.id);
    if (!fresh.length) return;
    setNewIds((prev) => new Set([...prev, ...fresh]));
    const id = setTimeout(() => setNewIds((prev) => {
      const next = new Set(prev);
      fresh.forEach((sig) => next.delete(sig));
      return next;
    }), 2400);
    return () => clearTimeout(id);
  }, [seer.SEED_SIGNALS]);

  // re-render for relative timestamps
  const [, force] = useState(0);
  useEffect(() => { const id = setInterval(() => force((x) => x + 1), 20000); return () => clearInterval(id); }, []);

  const shown = seer.SEED_SIGNALS.filter((s) => {
    if (dismissed.has(s.id)) return false;
    if (filter !== "ALL" && s.cat !== filter) return false;
    if (query) {
      const q = query.toLowerCase();
      return (s.head + s.body + s.proto.name).toLowerCase().includes(q);
    }
    return true;
  });

  const dismiss = (id) => setDismissed((prev) => new Set(prev).add(id));

  return (
    <div className="seer-screen">
      <header className="seer-screen-head">
        <div className="col" style={{ gap: 9, minWidth: 0 }}>
          <div className="row gap-12" style={{ flexWrap: "wrap", alignItems: "baseline" }}>
            <h1 className="serif seer-h1">Signal Feed</h1>
            <span className="row gap-6" style={{ whiteSpace: "nowrap" }}>
              <span className="dot live" />
              <span className="mono" style={{ fontSize: 12.5, color: "var(--ink-2)" }}>Live · <CountUp to={seer.stats.signalsToday} dur={900} /> signals today</span>
            </span>
          </div>
          <p className="seer-screen-sub" style={{ margin: 0 }}>On-chain intelligence, explained. Seer reads patterns the moment they form — not after.</p>
        </div>
        <button className={"btn btn-ghost" + (paused ? " seer-paused" : "")} onClick={() => setPaused((p) => !p)}>
          <Icon name={paused ? "signal" : "pause"} size={15} />{paused ? "Resume" : "Pause"} feed
        </button>
      </header>

      <div className="seer-feed-controls">
        <div className="seer-filters">
          {FILTERS.map((f) => (
            <button key={f} className={"seer-filter" + (filter === f ? " active" : "")} onClick={() => setFilter(f)}>
              {f !== "ALL" && <span className="seer-filter-dot" style={{ background: f === "ALL" ? "var(--ink-3s)" : CAT_STYLE[f].c }} />}
              {f}
            </button>
          ))}
        </div>
        <div className="seer-search">
          <Icon name="search" size={15} style={{ color: "var(--ink-3s)" }} />
          <input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Search wallets, protocols, tokens…" />
        </div>
      </div>

      <div className="seer-feed">
        {shown.length === 0 ? (
          seer.signalsLoading ? (
            <div className="seer-signals-loading">
              <div className="seer-spinner" />
              <span className="mono" style={{ fontSize: 13, color: "var(--ink-3s)", marginTop: 14 }}>Reading the chain…</span>
            </div>
          ) : query || filter !== "ALL" ? (
            <EmptyState icon="search" title="Nothing matches yet." body="No signals fit this filter right now. Seer is still watching — try ALL, or clear your search." cta="Clear filters" onCta={() => { setFilter("ALL"); setQuery(""); }} />
          ) : (
            <EmptyState icon="eye" title="Seer is watching." body="Signals will appear here the moment they are detected. Nothing escapes a patient eye." />
          )
        ) : (
          shown.map((s) => (
            <SignalCard key={s.id} s={s} isNew={newIds.has(s.id)} onMirror={onMirror} onDismiss={dismiss} />
          ))
        )}
      </div>
    </div>
  );
}
