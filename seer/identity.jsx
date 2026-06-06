/* ============================================================
   SEER — My Identity: oracle card + analysis + share
   ============================================================ */

function IdentityCardCanvas({ archKey, data, onReady, display = 340 }) {
  const ref = useRef(null);
  const H = display * (520 / 360);
  useEffect(() => {
    let cancelled = false;
    const draw = () => {
      if (cancelled) return;
      const cv = ref.current; if (!cv) return;
      const DPR = 2.4;
      cv.width = 360 * DPR; cv.height = 520 * DPR;
      const ctx = cv.getContext("2d");
      window.drawIdentityCard(ctx, DPR, archKey, data);
      onReady && onReady(cv);
    };
    if (document.fonts && document.fonts.ready) document.fonts.ready.then(draw); else draw();
    draw();
    return () => { cancelled = true; };
  }, [archKey, data.sbt.minted]);
  return <canvas ref={ref} className="seer-card-canvas" style={{ width: display, height: H }} />;
}

function ShareModal({ archKey, data, onClose }) {
  const [url, setUrl] = useState(null);
  const handleReady = useCallback((cv) => { setUrl(cv.toDataURL("image/png")); }, []);
  const arch = window.SEER.ARCHETYPES[archKey] || window.SEER.ARCHETYPES.strategist;
  const post = `My Mantle DeFi identity, read by @SeerProtocol: "${arch.name}" — ${data.percentileLabel}. See before they do. #MantleAIHackathon`;
  return (
    <div className="seer-modal-bg" onClick={onClose}>
      <div className="card seer-share-modal" onClick={(e) => e.stopPropagation()}>
        <div className="seer-trace-head" style={{ padding: "20px 22px" }}>
          <div className="col" style={{ gap: 3 }}>
            <span className="eyebrow">Share your reading</span>
            <span className="serif" style={{ fontSize: 19 }}>Ready for the timeline.</span>
          </div>
          <button className="btn-quiet" style={{ padding: 6 }} onClick={onClose}><Icon name="close" size={18} /></button>
        </div>
        <div className="seer-share-body">
          <div className="seer-share-preview">
            <IdentityCardCanvas archKey={archKey} data={data} onReady={handleReady} display={210} />
          </div>
          <div className="col gap-12" style={{ flex: 1, minWidth: 0 }}>
            <div className="seer-post-box">{post}</div>
            <div className="col gap-8">
              <button className="btn btn-primary" style={{ justifyContent: "center" }} onClick={() => window.open("https://twitter.com/intent/tweet?text=" + encodeURIComponent(post), "_blank")}>
                <Icon name="share" size={15} />Post to X
              </button>
              <a className="btn btn-ghost" style={{ justifyContent: "center" }} href={url || "#"} download="seer-identity.png">
                <Icon name="ext" size={15} />Download image
              </a>
            </div>
            <div className="faint" style={{ fontSize: 11.5 }}>The image renders identically in a tweet. Tested at 360×520, exported at 2.4×.</div>
          </div>
        </div>
      </div>
    </div>
  );
}

function ProtocolBar({ p }) {
  const max = 10;
  return (
    <div className="seer-proto-row">
      <span style={{ fontSize: 13, width: 120, flexShrink: 0 }}>{p.name}</span>
      <div className="seer-proto-track">
        <div className="seer-proto-fill bench" style={{ width: (p.smart / max) * 100 + "%" }} />
        <div className="seer-proto-fill you" style={{ width: (p.you / max) * 100 + "%" }} />
      </div>
      <span className="num" style={{ fontSize: 12.5, width: 74, textAlign: "right" }}>
        {p.you}% <span className="faint">/ {p.smart}%</span>
      </span>
    </div>
  );
}

function IdentityScreen({ showToast }) {
  const seer = window.useSeerStore();
  const base = seer.IDENTITY;
  const [minting, setMinting] = useState(false);
  const [share, setShare] = useState(false);
  const archKey = base.archetype;
  const A = window.SEER.ARCHETYPES[archKey] || window.SEER.ARCHETYPES.strategist;
  const data = base;

  const mint = async () => {
    setMinting(true);
    try {
      const identity = await window.SeerAPI.mintIdentity();
      showToast(identity.sbt.minted ? "Identity mint metadata ready." : "Mint metadata prepared; contract not configured yet.");
    } catch (err) {
      showToast(err.message || "Mint failed.");
    } finally {
      setMinting(false);
    }
  };

  if (!base.wallet) {
    return (
      <div className="seer-screen seer-screen-wide">
        <header className="seer-screen-head">
          <div className="col" style={{ gap: 9 }}>
            <h1 className="serif seer-h1">My Identity</h1>
            <p className="seer-screen-sub" style={{ margin: 0 }}>Seer read your wallet and named what it found.</p>
          </div>
        </header>
        <EmptyState icon="identity" title="Identity unavailable." body="The backend has not returned an identity payload for this wallet yet." />
      </div>
    );
  }

  return (
    <div className="seer-screen seer-screen-wide">
      <header className="seer-screen-head">
        <div className="col" style={{ gap: 9 }}>
          <h1 className="serif seer-h1">My Identity</h1>
          <p className="seer-screen-sub" style={{ margin: 0 }}>Seer read your wallet and named what it found. This is your DeFi identity — benchmarked against smart money, yours to mint and share.</p>
        </div>
      </header>

      {/* card + actions */}
      <section className="seer-identity-top">
        <div className="seer-card-frame">
          <IdentityCardCanvas archKey={archKey} data={data} display={344} />
        </div>
        <div className="seer-identity-side">
          <span className="eyebrow">Your reading</span>
          <div className="serif" style={{ fontSize: 28, lineHeight: 1.1, margin: "10px 0 4px" }}>{A.name}</div>
          <p className="mut" style={{ fontSize: 14, lineHeight: 1.6, margin: "0 0 18px", maxWidth: 380 }}>{A.reading}</p>

          <div className="seer-percentile-card">
            <RiskRingMini value={base.percentile} />
            <div className="col" style={{ gap: 2 }}>
              <span className="num" style={{ fontSize: 22, fontWeight: 600 }}>Top {base.percentile}%</span>
              <span className="faint" style={{ fontSize: 12 }}>of Mantle yield farmers</span>
            </div>
          </div>

          <div className="col gap-8" style={{ marginTop: 18 }}>
            {base.sbt.minted ? (
              <div className="seer-minted-chip"><Icon name="check" size={15} style={{ color: "var(--c-opp)" }} />Minted · Token #{base.sbt.token}</div>
            ) : (
              <button className="btn btn-primary" style={{ justifyContent: "center" }} disabled={minting} onClick={mint}>
                {minting ? <><span className="dot live" style={{ background: "#fff" }} />Minting…</> : <>Mint your identity<Icon name="arrow" size={16} /></>}
              </button>
            )}
            <button className="btn btn-ghost" style={{ justifyContent: "center" }} onClick={() => setShare(true)}>
              <Icon name="share" size={15} />Share identity
            </button>
          </div>
        </div>
      </section>

      {/* analysis */}
      <section className="seer-analysis">
        <div className="card seer-analysis-block">
          <div className="row" style={{ justifyContent: "space-between", marginBottom: 14 }}>
            <span className="eyebrow">Performance vs. smart money</span>
            <span className="num faint" style={{ fontSize: 12 }}>40d</span>
          </div>
          {seer.PERF.you.length && seer.PERF.bench.length ? <LineChart a={seer.PERF.you} b={seer.PERF.bench} w={560} h={170} /> : <EmptyState icon="signal" title="No performance series." body="The backend has not returned chart data for this identity." />}
        </div>

        <div className="card seer-analysis-block">
          <span className="eyebrow">Protocol breakdown · your APY vs. cohort</span>
          <div className="col gap-10" style={{ marginTop: 14 }}>
            {base.protocols.length === 0 ? <EmptyState icon="signal" title="No protocol breakdown." body="Protocol benchmark data is not available from the backend yet." /> : base.protocols.map((p) => <ProtocolBar key={p.name} p={p} />)}
          </div>
          <div className="row gap-16" style={{ marginTop: 14 }}>
            <span className="row gap-6 faint" style={{ fontSize: 11.5 }}><i style={{ width: 12, height: 8, background: "var(--coral)", borderRadius: 2 }} />You</span>
            <span className="row gap-6 faint" style={{ fontSize: 11.5 }}><i style={{ width: 12, height: 8, background: "var(--sky)", borderRadius: 2, opacity: 0.4 }} />Smart money</span>
          </div>
        </div>

        <div className="card seer-analysis-block" style={{ gridColumn: "1 / -1" }}>
          <span className="eyebrow">What Seer sees</span>
          <div className="seer-insights">
            {base.insights.length === 0 ? <EmptyState icon="eye" title="No insights yet." body="Seer did not return identity insights for this wallet." /> : base.insights.map((t, i) => (
              <div key={i} className="seer-insight" style={{ animation: `fadeUp .5s var(--ease-out) ${i * 0.1}s both` }}>
                <span className="center seer-insight-ic"><Icon name="eye" size={14} /></span>
                <span style={{ fontSize: 13.5, lineHeight: 1.55 }}>{t}</span>
              </div>
            ))}
          </div>
          <div className="seer-nextmove">
            <div className="col" style={{ gap: 4 }}>
              <span className="eyebrow" style={{ color: "var(--coral)" }}>Recommended next move</span>
              <span style={{ fontSize: 14, lineHeight: 1.5, maxWidth: 620 }}>{base.nextMove}</span>
            </div>
            <button className="btn btn-primary" style={{ flexShrink: 0 }} onClick={() => showToast("Routed to My Agent — intent pre-filled.")}>Let my agent execute<Icon name="arrow" size={15} /></button>
          </div>
        </div>
      </section>

      {share && <ShareModal archKey={archKey} data={data} onClose={() => setShare(false)} />}
    </div>
  );
}

/* small percentile ring */
function RiskRingMini({ value }) {
  const r = 22, c = 2 * Math.PI * r;
  const [v, setV] = useState(0);
  useEffect(() => { const id = setTimeout(() => setV(value), 150); return () => clearTimeout(id); }, []);
  return (
    <svg width="56" height="56" viewBox="0 0 56 56">
      <circle cx="28" cy="28" r={r} fill="none" stroke="var(--track)" strokeWidth="5" />
      <circle cx="28" cy="28" r={r} fill="none" stroke="var(--coral)" strokeWidth="5" strokeLinecap="round"
        strokeDasharray={c} strokeDashoffset={c * (v / 100)} transform="rotate(-90 28 28)"
        style={{ transition: "stroke-dashoffset 1s var(--ease)" }} />
    </svg>
  );
}

window.IdentityScreen = IdentityScreen;
