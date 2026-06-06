/* ============================================================
   SEER — Three Identity Directions
   ============================================================ */
const { useState } = React;

/* ---------- Logo marks (simple geometry only) ---------- */

// A · ORACLE — the eye / vesica lens
function MarkEye({ s = 64, ink = "#17150F", iris = "#E0431C", stroke = 4 }) {
  return (
    <svg width={s} height={s} viewBox="0 0 100 100" fill="none">
      <path d="M4 50 C 26 22, 74 22, 96 50 C 74 78, 26 78, 4 50 Z"
        stroke={ink} strokeWidth={stroke} strokeLinejoin="round" />
      <circle cx="50" cy="50" r="15" fill={iris} />
      <circle cx="50" cy="50" r="5" fill={ink} />
      <circle cx="44" cy="44" r="2.4" fill="#fff" opacity="0.9" />
    </svg>
  );
}

// B · INSTRUMENT — the aperture / iris
function MarkAperture({ s = 64, ink = "#EDEFF2", accent = "#C8F230", stroke = 3.4 }) {
  // pointy-top hexagon, r=30 outer, r=15 inner
  const outer = [[50,20],[76,35],[76,65],[50,80],[24,65],[24,35]];
  const inner = [[50,35],[63,42.5],[63,57.5],[50,65],[37,57.5],[37,42.5]];
  const innerPts = inner.map(p => p.join(",")).join(" ");
  return (
    <svg width={s} height={s} viewBox="0 0 100 100" fill="none">
      <circle cx="50" cy="50" r="40" stroke={ink} strokeWidth="1.4" opacity="0.5" />
      {outer.map((o, i) => (
        <line key={i} x1={o[0]} y1={o[1]} x2={inner[i][0]} y2={inner[i][1]}
          stroke={ink} strokeWidth={stroke} strokeLinecap="round" />
      ))}
      <polygon points={innerPts} stroke={accent} strokeWidth={stroke} strokeLinejoin="round" fill={accent + "1f"} />
    </svg>
  );
}

// C · PRISM — refraction, white light split into a spectrum
const SPECTRUM = ["#2F6BFF", "#16B981", "#F2A43B", "#D6409F"];
function MarkPrism({ s = 64, ink = "#0E1116", stroke = 3.6 }) {
  return (
    <svg width={s} height={s} viewBox="0 0 100 100" fill="none">
      <polygon points="50,16 82,74 18,74" stroke={ink} strokeWidth={stroke} strokeLinejoin="round" />
      <line x1="2" y1="56" x2="40" y2="56" stroke={ink} strokeWidth={stroke} strokeLinecap="round" />
      {SPECTRUM.map((c, i) => (
        <line key={i} x1={59} y1={50 + i * 3} x2={99} y2={36 + i * 11}
          stroke={c} strokeWidth={stroke} strokeLinecap="round" />
      ))}
    </svg>
  );
}

/* ---------- small shared bits ---------- */
function Swatch({ bg, name, hex, fg = "#fff", border }) {
  return (
    <div className="sw" style={{ background: bg, color: fg, borderRight: border }}>
      <div className="nm">{name}</div>
      <div className="hex">{hex}</div>
    </div>
  );
}

/* =====================================================================
   BOARD A — ORACLE
   ===================================================================== */
function BoardOracle() {
  const ink = "#17150F", paper = "#F4F1EA", verm = "#E0431C", ink2 = "#6E6657";
  return (
    <div className="board" style={{ background: paper, color: ink, fontFamily: '"Helvetica Neue", Helvetica, Arial, sans-serif' }}>
      <div className="bd-pad" style={{ paddingBottom: 24 }}>
        <div className="rowx" style={{ justifyContent: "space-between" }}>
          <span className="divlabel" style={{ color: verm }}>A · Oracle</span>
          <span className="divlabel" style={{ color: ink2 }}>Editorial intelligence</span>
        </div>
        <p style={{ fontFamily: '"Instrument Serif", serif', fontSize: 30, lineHeight: 1.06, margin: "18px 0 0", maxWidth: 560, letterSpacing: "-0.01em" }}>
          A house of foresight. Warm paper, sharp ink, one signal that burns.
        </p>
      </div>

      {/* hero lockup */}
      <div style={{ background: "#fff", borderTop: "1px solid #E3DECF", borderBottom: "1px solid #E3DECF", padding: "44px 44px" }}>
        <div className="rowx" style={{ gap: 18 }}>
          <MarkEye s={62} ink={ink} iris={verm} />
          <div style={{ fontFamily: '"Instrument Serif", serif', fontSize: 64, lineHeight: 0.8, letterSpacing: "0.01em" }}>Seer</div>
        </div>
        <div className="rowx" style={{ gap: 14, marginTop: 22 }}>
          <div style={{ width: 30, height: 1, background: verm }}></div>
          <span style={{ fontFamily: '"JetBrains Mono", monospace', fontSize: 12, letterSpacing: "0.18em", textTransform: "uppercase", color: ink2 }}>See before they do</span>
        </div>
      </div>

      {/* palette */}
      <div className="swrow">
        <Swatch bg={ink} name="Ink" hex="#17150F" border="1px solid #2a261c" />
        <Swatch bg={paper} name="Bone" hex="#F4F1EA" fg={ink} border="1px solid #E3DECF" />
        <Swatch bg={verm} name="Signal" hex="oklch .62 .21 33" />
        <Swatch bg="#C9BFA6" name="Clay" hex="#C9BFA6" fg={ink} />
      </div>

      {/* type + app */}
      <div className="bd-pad" style={{ display: "flex", gap: 30, flex: 1 }}>
        <div className="stack" style={{ flex: 1, gap: 14 }}>
          <div style={{ fontFamily: '"Instrument Serif", serif', fontSize: 58, lineHeight: 0.86 }}>Aa</div>
          <div>
            <div style={{ fontFamily: '"JetBrains Mono", monospace', fontSize: 10, letterSpacing: "0.14em", color: ink2, textTransform: "uppercase" }}>Instrument Serif · display</div>
            <div style={{ fontFamily: '"JetBrains Mono", monospace', fontSize: 10, letterSpacing: "0.14em", color: ink2, textTransform: "uppercase", marginTop: 6 }}>Helvetica Neue · interface</div>
            <div style={{ fontFamily: '"JetBrains Mono", monospace', fontSize: 10, letterSpacing: "0.14em", color: ink2, textTransform: "uppercase", marginTop: 6 }}>JetBrains Mono · data</div>
          </div>
        </div>
        {/* signal card */}
        <div className="appcard" style={{ flex: 1.15, background: "#fff", border: "1px solid #E3DECF", padding: 18, boxShadow: "0 1px 0 #EFEADC" }}>
          <div className="rowx" style={{ gap: 8, marginBottom: 12 }}>
            <span style={{ fontFamily: '"JetBrains Mono", monospace', fontSize: 9.5, letterSpacing: "0.14em", color: verm, background: "#FBE7DF", padding: "3px 7px", borderRadius: 5, textTransform: "uppercase" }}>Alpha</span>
            <span style={{ fontFamily: '"JetBrains Mono", monospace', fontSize: 9.5, letterSpacing: "0.06em", color: ink2 }}>Agni Finance · 4m</span>
          </div>
          <div style={{ fontFamily: '"Instrument Serif", serif', fontSize: 23, lineHeight: 1.04, marginBottom: 10 }}>Smart money entered Agni Pool #14</div>
          <div style={{ fontSize: 12.5, color: ink2, lineHeight: 1.45 }}>Three top-cohort wallets opened positions within nine minutes. Early rotation, not noise.</div>
          <div className="rowx" style={{ justifyContent: "space-between", marginTop: 16 }}>
            <span style={{ fontFamily: '"JetBrains Mono", monospace', fontSize: 11, color: ink }}>Conf · 86%</span>
            <span style={{ fontSize: 12, fontWeight: 600, color: "#fff", background: ink, padding: "7px 13px", borderRadius: 7 }}>Mirror this →</span>
          </div>
        </div>
      </div>
    </div>
  );
}

/* =====================================================================
   BOARD B — INSTRUMENT
   ===================================================================== */
function BoardInstrument() {
  const bg = "#0B0C0E", panel = "#15171B", line = "#23262B", ink = "#EDEFF2", ink2 = "#878D96", lime = "#C8F230";
  const mono = '"JetBrains Mono", monospace';
  return (
    <div className="board" style={{ background: bg, color: ink, fontFamily: mono }}>
      <div className="bd-pad" style={{ paddingBottom: 24 }}>
        <div className="rowx" style={{ justifyContent: "space-between" }}>
          <span className="divlabel" style={{ color: lime }}>B · Instrument</span>
          <span className="divlabel" style={{ color: ink2 }}>Quant terminal</span>
        </div>
        <p style={{ fontFamily: mono, fontSize: 18, lineHeight: 1.3, margin: "18px 0 0", maxWidth: 540, color: ink, letterSpacing: "-0.01em" }}>
          A precision instrument. Graphite, hairlines, and one current running through it.
        </p>
      </div>

      <div style={{ background: panel, borderTop: "1px solid " + line, borderBottom: "1px solid " + line, padding: "44px 44px" }}>
        <div className="rowx" style={{ gap: 18 }}>
          <MarkAperture s={60} ink={ink} accent={lime} />
          <div style={{ fontFamily: '"Archivo", sans-serif', fontWeight: 800, fontSize: 52, lineHeight: 0.8, letterSpacing: "-0.03em", textTransform: "uppercase" }}>Seer</div>
        </div>
        <div className="rowx" style={{ gap: 14, marginTop: 24 }}>
          <span style={{ width: 7, height: 7, borderRadius: 99, background: lime, boxShadow: "0 0 10px " + lime }}></span>
          <span style={{ fontFamily: mono, fontSize: 12, letterSpacing: "0.16em", textTransform: "uppercase", color: ink2 }}>See before they do</span>
        </div>
      </div>

      <div className="swrow">
        <Swatch bg={bg} name="Graphite" hex="#0B0C0E" border="1px solid #1c1f24" />
        <Swatch bg={panel} name="Panel" hex="#15171B" border="1px solid #1c1f24" />
        <Swatch bg={lime} name="Voltage" hex="oklch .89 .2 128" fg="#10120A" />
        <Swatch bg="#3DD2C0" name="Cyan" hex="#3DD2C0" fg="#0B0C0E" />
      </div>

      <div className="bd-pad" style={{ display: "flex", gap: 30, flex: 1 }}>
        <div className="stack" style={{ flex: 1, gap: 14 }}>
          <div style={{ fontFamily: '"Archivo", sans-serif', fontWeight: 900, fontSize: 58, lineHeight: 0.84, letterSpacing: "-0.03em" }}>Aa</div>
          <div>
            <div style={{ fontFamily: mono, fontSize: 10, letterSpacing: "0.12em", color: ink2, textTransform: "uppercase" }}>Archivo Black · display</div>
            <div style={{ fontFamily: mono, fontSize: 10, letterSpacing: "0.12em", color: ink2, textTransform: "uppercase", marginTop: 6 }}>JetBrains Mono · voice + data</div>
          </div>
        </div>
        <div className="appcard" style={{ flex: 1.15, background: panel, border: "1px solid " + line, padding: 18, borderRadius: 12 }}>
          <div className="rowx" style={{ gap: 8, marginBottom: 12 }}>
            <span style={{ fontFamily: mono, fontSize: 9.5, letterSpacing: "0.12em", color: lime, border: "1px solid " + lime + "55", padding: "3px 7px", borderRadius: 4, textTransform: "uppercase" }}>Alpha</span>
            <span style={{ fontFamily: mono, fontSize: 9.5, letterSpacing: "0.04em", color: ink2 }}>AGNI · 4M AGO</span>
          </div>
          <div style={{ fontFamily: '"Archivo", sans-serif', fontWeight: 700, fontSize: 19, lineHeight: 1.08, marginBottom: 10, letterSpacing: "-0.01em" }}>SMART MONEY ENTERED AGNI POOL #14</div>
          <div style={{ fontSize: 11.5, color: ink2, lineHeight: 1.5 }}>Three top-cohort wallets opened positions within nine minutes. Early rotation, not noise.</div>
          <div style={{ height: 6, background: "#1c1f24", borderRadius: 99, marginTop: 14, overflow: "hidden" }}>
            <div style={{ width: "86%", height: "100%", background: lime }}></div>
          </div>
          <div className="rowx" style={{ justifyContent: "space-between", marginTop: 14 }}>
            <span style={{ fontFamily: mono, fontSize: 11, color: ink }}>CONF 86%</span>
            <span style={{ fontFamily: mono, fontSize: 11, fontWeight: 700, color: "#10120A", background: lime, padding: "7px 13px", borderRadius: 6, textTransform: "uppercase" }}>Mirror ▸</span>
          </div>
        </div>
      </div>
    </div>
  );
}

/* =====================================================================
   BOARD C — PRISM
   ===================================================================== */
function BoardPrism() {
  const paper = "#FBFBFC", surf = "#F1F2F4", line = "#E4E6EA", ink = "#0E1116", ink2 = "#5C636D";
  const sans = '"Schibsted Grotesk", sans-serif';
  return (
    <div className="board" style={{ background: paper, color: ink, fontFamily: sans }}>
      <div className="bd-pad" style={{ paddingBottom: 24 }}>
        <div className="rowx" style={{ justifyContent: "space-between" }}>
          <span className="divlabel" style={{ color: ink }}>C · Prism</span>
          <span className="divlabel" style={{ color: ink2 }}>Signal, resolved</span>
        </div>
        <p style={{ fontFamily: sans, fontWeight: 500, fontSize: 21, lineHeight: 1.22, margin: "18px 0 0", maxWidth: 560, letterSpacing: "-0.015em" }}>
          Seer resolves on-chain noise into clear signal — like light through a prism. Near-mono, with a spectrum that means something.
        </p>
      </div>

      <div style={{ background: "#fff", borderTop: "1px solid " + line, borderBottom: "1px solid " + line, padding: "44px 44px" }}>
        <div className="rowx" style={{ gap: 18 }}>
          <MarkPrism s={58} ink={ink} />
          <div style={{ fontFamily: sans, fontWeight: 700, fontSize: 54, lineHeight: 0.8, letterSpacing: "-0.04em" }}>Seer</div>
        </div>
        <div style={{ height: 3, width: 220, marginTop: 24, borderRadius: 99, background: "linear-gradient(90deg," + SPECTRUM.join(",") + ")" }}></div>
        <div style={{ fontFamily: '"JetBrains Mono", monospace', fontSize: 12, letterSpacing: "0.16em", textTransform: "uppercase", color: ink2, marginTop: 12 }}>See before they do</div>
      </div>

      <div className="swrow">
        <Swatch bg={ink} name="Ink" hex="#0E1116" border="1px solid #1b1f26" />
        <Swatch bg={paper} name="Paper" hex="#FBFBFC" fg={ink} border={"1px solid " + line} />
        <Swatch bg={SPECTRUM[0]} name="Alpha" hex="#2F6BFF" />
        <Swatch bg={SPECTRUM[1]} name="Opp" hex="#16B981" />
        <Swatch bg={SPECTRUM[2]} name="Risk" hex="#F2A43B" fg={ink} />
        <Swatch bg={SPECTRUM[3]} name="Anomaly" hex="#D6409F" />
      </div>

      <div className="bd-pad" style={{ display: "flex", gap: 30, flex: 1 }}>
        <div className="stack" style={{ flex: 1, gap: 14 }}>
          <div style={{ fontFamily: sans, fontWeight: 700, fontSize: 58, lineHeight: 0.84, letterSpacing: "-0.04em" }}>Aa</div>
          <div>
            <div style={{ fontFamily: '"JetBrains Mono", monospace', fontSize: 10, letterSpacing: "0.12em", color: ink2, textTransform: "uppercase" }}>Schibsted Grotesk · display + UI</div>
            <div style={{ fontFamily: '"JetBrains Mono", monospace', fontSize: 10, letterSpacing: "0.12em", color: ink2, textTransform: "uppercase", marginTop: 6 }}>JetBrains Mono · data</div>
          </div>
        </div>
        <div className="appcard" style={{ flex: 1.15, background: "#fff", border: "1px solid " + line, padding: 18, position: "relative", overflow: "hidden" }}>
          <div style={{ position: "absolute", left: 0, top: 0, bottom: 0, width: 3, background: SPECTRUM[0] }}></div>
          <div className="rowx" style={{ gap: 8, marginBottom: 12 }}>
            <span style={{ fontFamily: '"JetBrains Mono", monospace', fontSize: 9.5, letterSpacing: "0.1em", color: "#fff", background: SPECTRUM[0], padding: "3px 7px", borderRadius: 5, textTransform: "uppercase" }}>Alpha</span>
            <span style={{ fontFamily: '"JetBrains Mono", monospace', fontSize: 9.5, letterSpacing: "0.04em", color: ink2 }}>Agni Finance · 4m</span>
          </div>
          <div style={{ fontFamily: sans, fontWeight: 600, fontSize: 19, lineHeight: 1.1, marginBottom: 10, letterSpacing: "-0.02em" }}>Smart money entered Agni Pool #14</div>
          <div style={{ fontSize: 12, color: ink2, lineHeight: 1.5 }}>Three top-cohort wallets opened positions within nine minutes. Early rotation, not noise.</div>
          <div className="rowx" style={{ justifyContent: "space-between", marginTop: 16 }}>
            <span style={{ fontFamily: '"JetBrains Mono", monospace', fontSize: 11, color: ink }}>Conf · 86%</span>
            <span style={{ fontSize: 12, fontWeight: 600, color: "#fff", background: ink, padding: "7px 13px", borderRadius: 8 }}>Mirror this →</span>
          </div>
        </div>
      </div>
    </div>
  );
}

/* =====================================================================
   Canvas
   ===================================================================== */
function App() {
  return (
    <DesignCanvas>
      <DCSection id="dirs" title="Seer — Identity Directions" subtitle="Three ways out of the generic. Pick one and I'll build the full system + deck. ★ = my recommendation.">
        <DCArtboard id="oracle" label="A · Oracle — editorial" width={760} height={1080}><BoardOracle /></DCArtboard>
        <DCArtboard id="instrument" label="B · Instrument — quant terminal" width={760} height={1080}><BoardInstrument /></DCArtboard>
        <DCArtboard id="prism" label="C · Prism — signal resolved ★" width={780} height={1080}><BoardPrism /></DCArtboard>
      </DCSection>
    </DesignCanvas>
  );
}

ReactDOM.createRoot(document.getElementById("root")).render(<App />);
