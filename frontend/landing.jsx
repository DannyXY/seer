/* ============================================================
   SEER — Landing (Instrument revamp)
   The prism mechanic is the hero: raw on-chain noise enters,
   the signal spectrum emerges. Terminal density, mono voice,
   voltage used once.
   ============================================================ */

const SPECTRUM = [
  { key: "alpha",   hex: "#4E9BFF", name: "Alpha",       ds: "Smart-money rotation" },
  { key: "opp",     hex: "#2BD4BE", name: "Opportunity", ds: "Mispricings & yield" },
  { key: "risk",    hex: "#F7A833", name: "Risk",        ds: "Exposure building" },
  { key: "anomaly", hex: "#E15CD2", name: "Anomaly",     ds: "Outliers, unclassified" },
];

const RAW_EVENTS = [
  "swap 12.4 MNT → USDC", "add_liquidity AGNI #14", "borrow 2,000 USDY",
  "0x8c1f… stake mETH", "vote incentive_gauge", "transfer 84,200 USDC",
  "0x5aea… open position", "claim 318 COOK", "repay 1,140 USDe",
  "0x2b9d… bridge in", "redeem 9.2 mETH", "approve PENDLE router",
];

/* ---------- Prism refraction field (signature hero motif) ---------- */
function PrismField() {
  const ref = useRef(null);
  useEffect(() => {
    const cv = ref.current; if (!cv) return;
    const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const ctx = cv.getContext("2d");
    const DPR = Math.min(2, window.devicePixelRatio || 1);
    let raf, t = 0, parts = [];
    function size() {
      const r = cv.getBoundingClientRect();
      cv.width = Math.max(1, r.width * DPR); cv.height = Math.max(1, r.height * DPR);
      ctx.setTransform(DPR, 0, 0, DPR, 0, 0);
    }
    size();
    const ro = new ResizeObserver(size); ro.observe(cv);

    const angles = [-0.36, -0.12, 0.12, 0.36];
    function spawn(w, h) {
      parts.push({
        x: -10, y: h * 0.5 + (Math.random() - 0.5) * h * 0.06,
        v: 0.8 + Math.random() * 0.9, band: -1, col: "237,239,242",
        a: 0, life: 0,
      });
    }
    function frame() {
      const r = cv.getBoundingClientRect();
      const w = r.width, h = r.height;
      const px = w * 0.46, py = h * 0.5;           // prism center
      const half = Math.min(w, h) * 0.17;          // prism size
      ctx.clearRect(0, 0, w, h);

      // faint grid
      ctx.strokeStyle = "rgba(150,160,182,0.045)"; ctx.lineWidth = 1;
      for (let gx = 0; gx < w; gx += 34) { ctx.beginPath(); ctx.moveTo(gx, 0); ctx.lineTo(gx, h); ctx.stroke(); }
      for (let gy = 0; gy < h; gy += 34) { ctx.beginPath(); ctx.moveTo(0, gy); ctx.lineTo(w, gy); ctx.stroke(); }

      // input beam guide
      ctx.strokeStyle = "rgba(237,239,242,0.16)"; ctx.lineWidth = 1.4;
      ctx.beginPath(); ctx.moveTo(0, py); ctx.lineTo(px - half * 0.5, py); ctx.stroke();

      // output ray guides + end labels
      const rayLen = w - px;
      ctx.textAlign = "right";
      angles.forEach((ang, i) => {
        const ex = px + Math.cos(ang) * rayLen, ey = py + Math.sin(ang) * rayLen;
        const g = ctx.createLinearGradient(px, py, ex, ey);
        g.addColorStop(0, hexA(SPECTRUM[i].hex, 0.5));
        g.addColorStop(1, hexA(SPECTRUM[i].hex, 0.06));
        ctx.strokeStyle = g; ctx.lineWidth = 1.4;
        ctx.beginPath(); ctx.moveTo(px + half * 0.4, py); ctx.lineTo(ex, ey); ctx.stroke();
        // label — right-aligned at a fixed inset so long names never clip
        const lx = w - 14, ly = py + Math.tan(ang) * (lx - px);
        ctx.fillStyle = hexA(SPECTRUM[i].hex, 0.92);
        ctx.font = "600 10px 'JetBrains Mono', monospace";
        ctx.fillText(SPECTRUM[i].name.toUpperCase(), lx, ly - 7);
      });
      ctx.textAlign = "left";

      // prism triangle
      ctx.beginPath();
      ctx.moveTo(px, py - half);
      ctx.lineTo(px + half * 0.86, py + half);
      ctx.lineTo(px - half * 0.86, py + half);
      ctx.closePath();
      ctx.strokeStyle = "rgba(237,239,242,0.55)"; ctx.lineWidth = 2;
      ctx.stroke();
      ctx.fillStyle = "rgba(200,242,48,0.04)"; ctx.fill();

      // particles
      if (!reduce && t % 7 === 0) spawn(w, h);
      parts = parts.filter((p) => {
        if (p.band === -1) {
          p.x += p.v;
          if (p.x >= px) { // refract
            p.band = Math.floor(Math.random() * 4);
            p.a = angles[p.band];
            p.col = hexRGB(SPECTRUM[p.band].hex);
          }
          ctx.fillStyle = `rgba(${p.col},0.85)`;
          ctx.fillRect(p.x, p.y, 2.4, 2.4);
        } else {
          p.x += Math.cos(p.a) * (p.v + 0.5);
          p.y = py + Math.sin(p.a) * (p.x - px);
          p.life += 1;
          ctx.beginPath(); ctx.arc(p.x, p.y, 1.9, 0, Math.PI * 2);
          ctx.fillStyle = `rgba(${p.col},0.95)`; ctx.fill();
          ctx.beginPath(); ctx.arc(p.x, p.y, 5, 0, Math.PI * 2);
          ctx.fillStyle = `rgba(${p.col},0.10)`; ctx.fill();
        }
        return p.x < w + 12 && p.y > -12 && p.y < h + 12;
      });

      t += 1;
      if (!reduce) raf = requestAnimationFrame(frame);
      else { /* draw a single static frame */ }
    }
    frame();
    return () => { cancelAnimationFrame(raf); ro.disconnect(); };
  }, []);
  return <canvas ref={ref} />;
}
function hexRGB(hex) {
  const n = parseInt(hex.slice(1), 16);
  return `${(n >> 16) & 255},${(n >> 8) & 255},${n & 255}`;
}
function hexA(hex, a) { return `rgba(${hexRGB(hex)},${a})`; }

/* ---------- scroll reveal (scroll-listener driven; bulletproof) ---------- */
function useReveal() {
  useEffect(() => {
    const els = Array.from(document.querySelectorAll(".sl-rise"));
    let done = false;
    const reveal = () => {
      const vh = window.innerHeight || document.documentElement.clientHeight;
      let remaining = 0;
      els.forEach((el) => {
        if (el.classList.contains("in")) return;
        if (el.getBoundingClientRect().top < vh * 0.9) el.classList.add("in");
        else remaining++;
      });
      if (remaining === 0) { done = true; window.removeEventListener("scroll", onScroll); }
    };
    const onScroll = () => { if (!done) requestAnimationFrame(reveal); };
    reveal();
    window.addEventListener("scroll", onScroll, { passive: true });
    window.addEventListener("resize", onScroll, { passive: true });
    // safety net: never leave anything hidden
    const safety = setTimeout(() => els.forEach((el) => el.classList.add("in")), 2600);
    return () => { window.removeEventListener("scroll", onScroll); window.removeEventListener("resize", onScroll); clearTimeout(safety); };
  }, []);
}

/* ---------- connect modal (kept; lightly restyled) ---------- */
const WALLETS = [
  { id: "mm", name: "MetaMask", tag: "Most popular", g: "🦊" },
  { id: "wc", name: "WalletConnect", tag: "Scan with phone", g: "◇" },
  { id: "cb", name: "Coinbase Wallet", tag: "", g: "◎" },
  { id: "email", name: "Continue with email", tag: "No wallet yet? Start here", g: "✉" },
];
function ConnectModal({ onClose, onConnect }) {
  const [stage, setStage] = useState("pick");
  const [chosen, setChosen] = useState(null);
  const [error, setError] = useState(null);
  const choose = async (w) => {
    setChosen(w);
    setStage("connecting");
    setError(null);
    try {
      await onConnect();
    } catch (err) {
      setError(err.message);
      setStage("pick");
    }
  };
  return (
    <div className="seer-modal-bg" onClick={onClose}>
      <div className="card" style={{ width: 400, padding: 28, animation: "fadeScale .28s var(--ease-out) both" }} onClick={(e) => e.stopPropagation()}>
        {stage === "pick" ? (
          <>
            <div className="row" style={{ justifyContent: "space-between", marginBottom: 6 }}>
              <div className="eyebrow">Connect</div>
              <button className="btn-quiet" style={{ padding: 4 }} onClick={onClose}><Icon name="close" size={16} /></button>
            </div>
            <div className="serif" style={{ fontSize: 24, marginBottom: 4, textTransform: "uppercase", letterSpacing: "-0.02em" }}>Step inside.</div>
            <div className="mut" style={{ fontSize: 13.5, marginBottom: 20 }}>One click. No email, no seed phrase to type. Just connect and see.</div>
            <div className="col gap-8">
              {WALLETS.map((w) => (
                <button key={w.id} className="seer-wallet-row" onClick={() => choose(w)}>
                  <span className="center" style={{ width: 34, height: 34, borderRadius: 9, background: "var(--card-2)", border: "1px solid var(--line)", fontSize: 17 }}>{w.g}</span>
                  <span className="col" style={{ alignItems: "flex-start", lineHeight: 1.3 }}>
                    <span style={{ fontWeight: 500, fontSize: 14 }}>{w.name}</span>
                    {w.tag && <span style={{ fontSize: 11.5, color: "var(--ink-3s)" }}>{w.tag}</span>}
                  </span>
                  <Icon name="chevR" size={16} style={{ marginLeft: "auto", color: "var(--ink-3s)" }} />
                </button>
              ))}
            </div>
            {error && <div className="mut" style={{ fontSize: 12.5, marginTop: 12, color: "var(--danger)" }}>{error}</div>}
            <div className="row gap-6 faint" style={{ fontSize: 11.5, marginTop: 16, justifyContent: "center" }}>
              <Icon name="shield" size={13} /> Seer never holds your funds. Permissions are scoped and revocable.
            </div>
          </>
        ) : (
          <div className="center" style={{ flexDirection: "column", padding: "26px 0", gap: 18 }}>
            <div className="center" style={{ width: 60, height: 60, borderRadius: 99, background: "var(--coral-wash)", color: "var(--coral)", fontSize: 26 }}>
              <span style={{ animation: "pulseSoft 1.2s var(--ease) infinite" }}>{chosen?.g}</span>
            </div>
            <div className="col center" style={{ gap: 4 }}>
              <div className="serif" style={{ fontSize: 20, textTransform: "uppercase", letterSpacing: "-0.02em" }}>Connecting to {chosen?.name}…</div>
              <div className="mut" style={{ fontSize: 13 }}>Approve the request in your wallet.</div>
            </div>
            <div className="cbar" style={{ width: 200 }}><i style={{ background: "var(--coral)", width: "100%", transition: "width 1.3s var(--ease)" }} /></div>
          </div>
        )}
      </div>
    </div>
  );
}

/* ---------- surfaces ---------- */
const SURFACES = [
  { n: "01", c: "#4E9BFF", nm: "Signal Feed",      ds: "On-chain intelligence, explained the moment it happens — with the wallets, timing, and confidence behind every call." },
  { n: "02", c: "#2BD4BE", nm: "The Agent",        ds: "Set an intent in plain English. Seer finds the route, executes on-chain, and rebalances — non-custodial, while you sleep." },
  { n: "03", c: "#E15CD2", nm: "The Arena",        ds: "Seer makes public predictions and stakes its reputation. Bet with it, or against it — win if you're sharper." },
  { n: "04", c: "#F7A833", nm: "On-chain Identity", ds: "Every call you make and move you mirror builds a portable, provable record of what you've gotten right." },
];

function Landing({ onEnter }) {
  const seer = window.useSeerStore();
  const [modal, setModal] = useState(false);
  const S = seer.stats;
  useReveal();
  return (
    <div className="sl">
      {/* nav */}
      <nav className="sl-nav">
        <div className="sl-wrap sl-nav-in">
          <div className="sl-brand"><PrismMark size={26} /><span className="wm">SEER</span></div>
          <div className="sl-nav-links">
            <a onClick={() => setModal(true)}>Signals</a><a onClick={() => setModal(true)}>Agent</a><a onClick={() => setModal(true)}>Arena</a><a onClick={() => setModal(true)}>Identity</a>
          </div>
          <div className="sl-nav-right">
            <span className="pill"><span className="dot live" style={{ background: "var(--volt)" }} />Mantle</span>
            <button className="btn btn-primary" onClick={() => setModal(true)}>Connect<Icon name="arrow" size={15} /></button>
          </div>
        </div>
      </nav>

      <div className="sl-wrap">
        {/* hero */}
        <section className="sl-hero">
          <div>
            <div className="eyebrow">On-chain intelligence · Built on Mantle</div>
            <h1 className="sl-hero-h1">See<br /><em>before</em><br />they do.</h1>
            <p className="sl-hero-sub">Seer reads Mantle the moment patterns form — then turns the noise into <b>signal you can act on in one click.</b> An instrument, not another dashboard.</p>
            <div className="sl-hero-cta">
              <button className="btn btn-primary" style={{ padding: "14px 22px", fontSize: 15 }} onClick={() => setModal(true)}>Connect Wallet<Icon name="arrow" size={17} /></button>
              <button className="btn btn-ghost" style={{ padding: "14px 18px", fontSize: 15 }} onClick={() => setModal(true)}>Enter app</button>
            </div>
            <div className="sl-reassure">// no sign-up · no email · just your wallet</div>
          </div>

          <div className="sl-inst">
            <div className="sl-inst-bar">
              <span className="dot live" style={{ background: "var(--volt)" }} />
              <span className="t" style={{ whiteSpace: "nowrap" }}>Prism · live</span>
              <span className="t" style={{ marginLeft: "auto", whiteSpace: "nowrap" }}>BLOCK #4,210,887</span>
            </div>
            <div className="sl-inst-canvas"><PrismField /></div>
            <div className="sl-inst-foot">
              <span className="t" style={{ color: "var(--ink-2)" }}>NOISE&nbsp;IN</span>
              {SPECTRUM.map((s) => (
                <span className="k" key={s.key}><i style={{ background: s.hex }} />{s.name}</span>
              ))}
            </div>
          </div>
        </section>

        {/* ticker */}
        <section className="sl-ticker">
          <div className="sl-tk"><span className="dot live" /><span className="l mono" style={{ letterSpacing: "0.06em" }}>LIVE</span></div>
          <span className="sl-tk-div" />
          <div className="sl-tk"><CountUp className="v" to={S.signalsToday} dur={1100} /><span className="l">signals generated</span></div>
          <span className="sl-tk-div" />
          <div className="sl-tk"><CountUp className="v" to={S.agentAssets} decimals={1} prefix="$" suffix="M" dur={1100} /><span className="l">in agent-managed assets</span></div>
          <span className="sl-tk-div" />
          <div className="sl-tk"><CountUp className="v" to={S.cardsMinted} dur={1100} /><span className="l">identity cards minted</span></div>
          <span className="sl-tk-div" />
          <div className="sl-tk"><span className="v" style={{ color: "var(--volt)" }}>74%</span><span className="l">prediction accuracy</span></div>
        </section>

        {/* mechanic */}
        <section className="sl-sec sl-rise">
          <div className="sl-sec-head">
            <span className="idx">01</span><h2>Noise in.<br />Signal out.</h2>
            <p>Thousands of wallets, pools and events cross Mantle every minute. Seer resolves them — like light through a prism — into four kinds of signal.</p>
          </div>
          <div className="sl-mech">
            <div className="sl-mech-col">
              <div className="cap">Raw on-chain events</div>
              <div className="sl-noise">{RAW_EVENTS.map((e, i) => <div key={i}>{e}</div>)}</div>
            </div>
            <div className="sl-prismbox"><PrismMark size={76} /></div>
            <div className="sl-mech-col">
              <div className="cap">Resolved signal</div>
              <div className="sl-spectrum">
                {SPECTRUM.map((s) => (
                  <div className="sl-band" key={s.key}>
                    <span className="sw" style={{ background: s.hex }} />
                    <span className="bt">
                      <span className="nm" style={{ color: s.hex }}>{s.name}</span>
                      <span className="ds">{s.ds}</span>
                    </span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </section>

        {/* product proof */}
        <section className="sl-sec sl-proof sl-rise">
          <div>
            <span className="idx mono" style={{ color: "var(--volt)", fontSize: 12, letterSpacing: "0.1em" }}>02</span>
            <h2>Every signal,<br />with its <em>evidence.</em></h2>
            <p>No black box. Each read names the wallets, the protocol, the timing and a confidence score — then offers one button to act.</p>
            <div className="sl-proof-list">
              <div className="it"><Icon name="check" size={17} /><span>Confidence-scored, never a bare alert</span></div>
              <div className="it"><Icon name="check" size={17} /><span>Linked to the exact wallets and txns</span></div>
              <div className="it"><Icon name="check" size={17} /><span>One-click mirror, executed on Mantle</span></div>
            </div>
          </div>
          <div className="sl-card">
            <div className="meta">
              <span className="badge alpha" style={{ color: "var(--c-alpha)", borderColor: "var(--c-alpha)", background: "var(--c-alpha-wash)" }}>Alpha</span>
              <span className="pill">◇ Agni Finance</span>
              <span className="eyebrow" style={{ marginLeft: "auto" }}>4 min ago</span>
            </div>
            <h4>Smart money entered Agni Pool #14</h4>
            <p>Three top-cohort wallets opened positions in the MNT/USDC pool within nine minutes. Early rotation, not noise.</p>
            <div className="foot">
              <div className="cbar grow"><i style={{ width: "86%", background: "var(--volt)" }} /></div>
              <span className="num" style={{ fontSize: 13, color: "var(--ink-2)" }}>86%</span>
              <button className="btn btn-primary" style={{ padding: "9px 15px" }}>Mirror<Icon name="arrow" size={15} /></button>
            </div>
          </div>
        </section>

        {/* surfaces */}
        <section className="sl-sec sl-rise">
          <div className="sl-sec-head">
            <span className="idx">03</span><h2>Four surfaces,<br />one engine.</h2>
            <p>See it, act on it, wager on it, and own the record. Each surface reads from the same live signal engine.</p>
          </div>
          <div className="sl-surf">
            {SURFACES.map((s) => (
              <div className="sl-surf-card" key={s.n}>
                <div className="top">
                  <span className="dotc" style={{ background: s.c }} />
                  <span className="num">{s.n}</span>
                </div>
                <div className="nm">{s.nm}</div>
                <div className="ds">{s.ds}</div>
                <span className="edge" style={{ background: s.c }} />
              </div>
            ))}
          </div>
        </section>
      </div>

      {/* close */}
      <div className="sl-wrap">
        <section className="sl-close sl-rise">
          <PrismMark size={64} style={{ margin: "0 auto", filter: "drop-shadow(0 0 36px rgba(200,242,48,0.14))" }} />
          <h2>The market is talking.<br /><em>Seer is listening.</em></h2>
          <div className="sl-close-cta">
            <button className="btn btn-primary" style={{ padding: "15px 26px", fontSize: 16 }} onClick={() => setModal(true)}>Connect Wallet<Icon name="arrow" size={17} /></button>
            <button className="btn btn-ghost" style={{ padding: "15px 22px", fontSize: 16 }} onClick={() => setModal(true)}>Enter app</button>
          </div>
        </section>

        <footer className="sl-foot">
          <div className="sl-brand"><PrismMark size={22} /><span className="wm" style={{ fontSize: 16 }}>SEER</span></div>
          <div className="row gap-24"><a>GitHub</a><a>X / Twitter</a><a>Telegram</a></div>
          <span className="pill"><span className="dot" style={{ background: "var(--c-opp)" }} />Mantle Network</span>
        </footer>
      </div>

      {modal && <ConnectModal onClose={() => setModal(false)} onConnect={onEnter} />}
    </div>
  );
}

window.Landing = Landing;
