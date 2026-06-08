/* ============================================================
   SEER - The Arena: predictions vs. AI + leaderboard
   ============================================================ */
import { useState, useRef, useEffect } from 'react';
import { sendOnChainTx } from '../utils/onchain.js';

function Odds({ conf }) {
  return (
    <div className="seer-odds">
      <div className="seer-odds-bar">
        <div className="seer-odds-seer" style={{ width: conf + "%" }}>
          <span>SEER {conf}%</span>
        </div>
        <div className="seer-odds-you" style={{ width: (100 - conf) + "%" }}>
          <span>YOU {100 - conf}%</span>
        </div>
      </div>
    </div>
  );
}

function PredictionCard({ p, onBet, tick }) {
  const hot = p.pool > 4000 || p.hot;
  const hiConf = p.conf >= 65;
  return (
    <article className={"seer-pred" + (hiConf ? " hi" : "")}>
      <div className="row gap-8" style={{ marginBottom: 12, flexWrap: "wrap" }}>
        <span className="badge" style={{ color: "var(--coral)", background: "var(--coral-wash)" }}><Icon name="logo" size={11} />Seer predicts</span>
        {hot && <span className="badge" style={{ color: "var(--c-risk)", background: "var(--c-risk-wash)" }}><Icon name="flame" size={11} />Hot</span>}
        <span className="grow" />
        <span className="row gap-5 mono" style={{ fontSize: 11.5, color: "var(--ink-2)" }}><Icon name="clock" size={13} />{countdown(p.ends)}</span>
      </div>

      <h3 className="seer-pred-claim">{p.claim}</h3>
      <p className="seer-pred-reason"><span style={{ color: "var(--coral-ink)", fontWeight: 500 }}>Seer's read - </span>{p.reason}</p>

      <Odds conf={p.conf} />

      <div className="row" style={{ justifyContent: "space-between", marginTop: 14, marginBottom: 14 }}>
        <span className="faint mono" style={{ fontSize: 11.5, whiteSpace: "nowrap" }}>Prize pool</span>
        <span className="num" style={{ fontSize: 13, fontWeight: 500, whiteSpace: "nowrap" }}><CountUp to={p.pool} dur={900} /> pts</span>
      </div>

      <div className="seer-bet-btns">
        <button className="seer-bet-agree" onClick={() => onBet(p, "AGREE")}>
          <Icon name="check" size={15} />Agree with Seer
        </button>
        <button className="seer-bet-against" onClick={() => onBet(p, "AGAINST")}>
          <Icon name="x" size={13} />Bet against
        </button>
      </div>
    </article>
  );
}

function BetModal({ pred, side, balance, onClose, onPlace }) {
  const [amt, setAmt] = useState(50);
  const odds = side === "AGREE" ? pred.conf : 100 - pred.conf;
  const payout = (amt * (100 / odds)).toFixed(1);
  const insufficient = amt > balance;
  return (
    <div className="seer-modal-bg" onClick={onClose}>
      <div className="card" style={{ width: 400, padding: 24, animation: "fadeScale .26s var(--ease-out) both" }} onClick={(e) => e.stopPropagation()}>
        <div className="row" style={{ justifyContent: "space-between", marginBottom: 12 }}>
          <span className="badge" style={{ color: side === "AGREE" ? "var(--c-opp)" : "var(--coral)", background: side === "AGREE" ? "var(--c-opp-wash)" : "var(--coral-wash)" }}>
            {side === "AGREE" ? "Agree with Seer" : "Bet against Seer"}
          </span>
          <button className="btn-quiet" style={{ padding: 4 }} onClick={onClose}><Icon name="close" size={16} /></button>
        </div>
        <p style={{ fontSize: 14, lineHeight: 1.45, margin: "0 0 18px" }}>{pred.claim}</p>

        <span className="eyebrow">Your stake (points)</span>
        <div className="seer-amount">
          <span className="num" style={{ fontSize: 26, fontWeight: 600 }}>{amt} <span style={{ fontSize: 15, color: "var(--ink-2)", fontWeight: 500 }}>pts</span></span>
          <input type="range" min="10" max="500" step="10" value={amt} onChange={(e) => setAmt(+e.target.value)} className="seer-range" />
        </div>
        <div className="row gap-8" style={{ marginTop: 10 }}>
          {[25, 50, 100, 250].map((q) => (
            <button key={q} className={"seer-chip" + (amt === q ? " active" : "")} onClick={() => setAmt(q)}>{q}</button>
          ))}
        </div>

        <div className="seer-payout">
          <div className="row" style={{ justifyContent: "space-between" }}><span className="mut" style={{ fontSize: 13 }}>Odds</span><span className="num" style={{ fontSize: 13 }}>{odds}%</span></div>
          <div className="row" style={{ justifyContent: "space-between", marginTop: 6 }}><span className="mut" style={{ fontSize: 13 }}>Potential payout</span><span className="num" style={{ fontSize: 15, fontWeight: 600, color: "var(--c-opp)" }}>{payout} pts</span></div>
        </div>

        <div className="row" style={{ justifyContent: "space-between", marginTop: 12 }}>
          <span className="faint" style={{ fontSize: 12.5 }}>Your balance</span>
          <span className="num" style={{ fontSize: 12.5, color: insufficient ? "var(--danger)" : "var(--ink-2)" }}>{balance} pts</span>
        </div>

        <button className="btn btn-primary" disabled={insufficient} style={{ width: "100%", justifyContent: "center", marginTop: 14, opacity: insufficient ? 0.5 : 1, cursor: insufficient ? "not-allowed" : "pointer" }} onClick={() => !insufficient && onPlace(pred, side, amt, payout)}>
          {insufficient ? "Not enough points" : <>Place bet · {amt} pts<Icon name="arrow" size={16} /></>}
        </button>
      </div>
    </div>
  );
}

const BET_STATUS = {
  ACTIVE: { c: "var(--ink-2)", bg: "var(--card-2)", label: "ACTIVE" },
  WIN: { c: "var(--c-opp)", bg: "var(--c-opp-wash)", label: "WIN" },
  LOSS: { c: "var(--danger)", bg: "var(--c-alpha-wash)", label: "LOSS" },
};
function BetRow({ b }) {
  const s = BET_STATUS[b.status];
  return (
    <div className="seer-betrow">
      <div className="col" style={{ gap: 5, minWidth: 0 }}>
        <span style={{ fontSize: 13, lineHeight: 1.35 }}>{b.claim}</span>
        <div className="row gap-8">
          <span className="badge" style={{ color: s.c, background: s.bg }}>{s.label}</span>
          <span className="faint mono" style={{ fontSize: 11 }}>{b.side} · {b.amount} pts</span>
        </div>
      </div>
      <div className="col" style={{ alignItems: "flex-end", flexShrink: 0 }}>
        {b.status === "ACTIVE"
          ? <span className="num faint" style={{ fontSize: 12.5 }}>→ {b.potential} pts</span>
          : <span className="num" style={{ fontSize: 13.5, fontWeight: 500, color: b.pnl >= 0 ? "var(--c-opp)" : "var(--danger)" }}>{b.pnl >= 0 ? "+" : "−"}{Math.abs(b.pnl).toFixed(1)} pts</span>}
      </div>
    </div>
  );
}


export function ArenaScreen({ showToast }) {
  const seer = window.useSeerStore();
  const [betting, setBetting] = useState(null);
  const [tab, setTab] = useState("bets");
  const [onchain, setOnchain] = useState(null); // { available_points, has_claimed_starter_points, claim_starter_calldata }
  const [claiming, setClaiming] = useState(false);
  const [txPending, setTxPending] = useState(false);
  const [, force] = useState(0);
  useEffect(() => { const id = setInterval(() => force((x) => x + 1), 1000); return () => clearInterval(id); }, []);

  // Load on-chain points state when the screen mounts
  useEffect(() => {
    window.SeerAPI.loadOnChainPoints().then(data => {
      if (!data) return;
      setOnchain(data);
      if (data.available_points > 0) {
        window.SEER.update({ userPoints: data.available_points });
      }
    });
  }, [seer.wallet]);

  const rec = seer.SEER_RECORD;

  const claimStarterPoints = async () => {
    if (!onchain?.claim_starter_calldata) return;
    setClaiming(true);
    try {
      const hash = await sendOnChainTx(onchain.claim_starter_calldata);
      showToast(`Claimed 1,000 pts - tx: ${hash.slice(0, 10)}…`, 'success');
      setOnchain(prev => ({ ...prev, has_claimed_starter_points: true, available_points: 1000 }));
      window.SEER.update({ userPoints: 1000 });
    } catch (err) {
      showToast(err.message || "Claim failed.", 'error');
    } finally {
      setClaiming(false);
    }
  };

  const place = async (pred, side, amt) => {
    try {
      const result = await window.SeerAPI.placeBet(pred.id, side, amt);
      setBetting(null); setTab("bets");

      if (result.entry_calldata) {
        setTxPending(true);
        try {
          const hash = await sendOnChainTx(result.entry_calldata);
          showToast(`Bet confirmed on-chain - tx: ${hash.slice(0, 10)}…`, 'success');
        } catch (txErr) {
          showToast(`Bet recorded. On-chain tx failed: ${txErr.message}`, 'error');
        } finally {
          setTxPending(false);
        }
      } else {
        showToast(side === "AGREE" ? "Bet placed - you're with Seer." : "Bet placed - you're against Seer. Bold.", 'success');
      }
    } catch (err) {
      showToast(err.message || "Bet failed.", 'error');
    }
  };

  return (
    <div className="seer-screen seer-screen-wide">
      <header className="seer-screen-head">
        <div className="col" style={{ gap: 9 }}>
          <h1 className="serif seer-h1">The Arena</h1>
          <p className="seer-screen-sub" style={{ margin: 0 }}>Seer makes on-chain predictions. Bet with it, or against it. Win if you're smarter - lose, and learn exactly why.</p>
        </div>
        <div className="seer-balance">
          <span className="eyebrow" style={{ whiteSpace: "nowrap" }}>Your balance</span>
          <span className="num" style={{ fontSize: 26, fontWeight: 600, lineHeight: 1, whiteSpace: "nowrap" }}>
            <span style={{ color: "var(--volt)" }}><CountUp to={seer.userPoints} dur={700} /></span>
            <span style={{ fontSize: 14, color: "var(--ink-2)", fontWeight: 500 }}> pts</span>
          </span>
        </div>
      </header>

      {onchain && !onchain.has_claimed_starter_points && onchain.claim_starter_calldata && (
        <div className="seer-claim-banner">
          <div className="col" style={{ gap: 3 }}>
            <span style={{ fontWeight: 600, fontSize: 14 }}>Claim your 1,000 starter points</span>
            <span className="mut" style={{ fontSize: 12.5 }}>One on-chain transaction mints your points to Mantle Sepolia. Required to place bets.</span>
          </div>
          <button className="btn btn-primary" style={{ flexShrink: 0, padding: "10px 18px" }} onClick={claimStarterPoints} disabled={claiming}>
            {claiming ? "Claiming…" : <>Claim 1,000 pts<Icon name="arrow" size={15} /></>}
          </button>
        </div>
      )}

      {txPending && (
        <div className="seer-claim-banner" style={{ borderColor: "var(--c-opp-line)", background: "var(--c-opp-wash)" }}>
          <div className="seer-spinner" style={{ width: 18, height: 18, borderTopColor: "var(--c-opp)", flexShrink: 0 }} />
          <span style={{ fontSize: 13.5 }}>Sending bet to Mantle Sepolia - confirm in your wallet…</span>
        </div>
      )}

      <div className="seer-arena-grid">
        {/* predictions */}
        <section className="col gap-16">
          <div className="row" style={{ justifyContent: "space-between" }}>
            <span className="eyebrow" style={{ whiteSpace: "nowrap" }}>Open predictions</span>
            <span className="faint num" style={{ fontSize: 12, whiteSpace: "nowrap" }}>{seer.PREDICTIONS.length} live</span>
          </div>
          <div className="seer-pred-list">
            {seer.PREDICTIONS.length === 0 ? (
              <EmptyState icon="arena" title="No open predictions." body="The backend has not published an Arena prediction yet." />
            ) : seer.PREDICTIONS.map((p) => <PredictionCard key={p.id} p={p} onBet={(pp, s) => setBetting({ pred: pp, side: s })} />)}
          </div>
        </section>

        {/* side: record + tabs */}
        <aside className="col gap-16">
          <div className="card seer-record">
            <span className="eyebrow">Seer's track record</span>
            <div className="row" style={{ alignItems: "baseline", gap: 8, margin: "10px 0 2px" }}>
              <span className="num" style={{ fontSize: 38, fontWeight: 600, color: "var(--coral)" }}><CountUp to={rec.accuracy} dur={1000} />%</span>
              <span className="mut" style={{ fontSize: 13 }}>accuracy</span>
            </div>
            <div className="faint mono" style={{ fontSize: 12 }}>{rec.total} predictions · {rec.correct} correct</div>
            <div className="seer-record-track">
              {Array.from({ length: rec.total }).map((_, i) => (
                <span key={i} className="seer-record-tick" style={{ background: i < rec.correct ? "var(--c-opp)" : "var(--line)" }} />
              ))}
            </div>
          </div>

          <div className="card" style={{ padding: 4 }}>
            <div className="seer-tabs">
              <button className={"seer-tab" + (tab === "bets" ? " active" : "")} onClick={() => setTab("bets")}>My bets</button>
              <button className={"seer-tab" + (tab === "board" ? " active" : "")} onClick={() => setTab("board")}>Leaderboard</button>
            </div>
            <div style={{ padding: "10px 14px 14px" }}>
              {tab === "bets" ? (
                seer.MY_BETS.length === 0
                  ? <EmptyState icon="arena" title="No bets yet." body="Pick a prediction and place your first bet against Seer." />
                  : <div className="col gap-10">{seer.MY_BETS.map((b) => <BetRow key={b.id} b={b} />)}</div>
              ) : (
                <div className="col gap-2">
                  {seer.LEADERBOARD.length === 0 ? <EmptyState icon="arena" title="No leaderboard yet." body="Entries will appear here once the backend has Arena participants." /> : seer.LEADERBOARD.map((r) => (
                    <div key={r.rank} className={"seer-lb-row" + (r.you ? " you" : "")}>
                      <span className="num faint" style={{ width: 22, fontSize: 12.5 }}>{r.rank}</span>
                      <span className="mono" style={{ fontSize: 12.5, flex: 1 }}>{window.SEER.util.shortAddr(r.addr)}{r.you && <span className="seer-you-tag">you</span>}</span>
                      <span className="num" style={{ fontSize: 12.5, fontWeight: 500, color: "var(--c-opp)" }}>+{r.pnl} pts</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        </aside>
      </div>

      {betting && <BetModal pred={betting.pred} side={betting.side} balance={seer.userPoints} onClose={() => setBetting(null)} onPlace={place} />}
    </div>
  );
}
