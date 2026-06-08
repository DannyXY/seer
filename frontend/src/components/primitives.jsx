/* ============================================================
   SEER - shared primitives
   ============================================================ */
import { useState, useEffect, useRef, useMemo, useCallback } from 'react';

/* ---------- Icons (simple line glyphs) ---------- */
function Icon({ name, size = 18, stroke = 1.6, style }) {
  const p = { fill: "none", stroke: "currentColor", strokeWidth: stroke, strokeLinecap: "round", strokeLinejoin: "round" };
  const paths = {
    signal: <><path {...p} d="M2 12c0-1 3-2 10-2s10 1 10 2-3 2-10 2S2 13 2 12Z"/><circle cx="12" cy="12" r="2.4" fill="currentColor" stroke="none"/><path {...p} d="M5.5 7.5C7 6.4 9.3 6 12 6M5.5 16.5C7 17.6 9.3 18 12 18"/></>,
    agent: <><path {...p} d="M13 2 4 13h6l-1 9 9-11h-6l1-9Z"/></>,
    identity: <><rect {...p} x="3" y="5" width="18" height="14" rx="2.4"/><circle {...p} cx="8.5" cy="11" r="2"/><path {...p} d="M14 9.5h4M14 13h4M5.5 15.5c.7-1.4 4-1.4 4.7 0"/></>,
    arena: <><circle {...p} cx="12" cy="12" r="9"/><circle {...p} cx="12" cy="12" r="4.6"/><circle cx="12" cy="12" r="1.6" fill="currentColor" stroke="none"/></>,
    settings: <><circle {...p} cx="12" cy="12" r="3"/><path {...p} d="M12 2v3M12 19v3M4.2 4.2l2.1 2.1M17.7 17.7l2.1 2.1M2 12h3M19 12h3M4.2 19.8l2.1-2.1M17.7 6.3l2.1-2.1"/></>,
    eye: <><path {...p} d="M2 12s3.6-6.5 10-6.5S22 12 22 12s-3.6 6.5-10 6.5S2 12 2 12Z"/><circle {...p} cx="12" cy="12" r="3"/></>,
    arrow: <><path {...p} d="M5 12h13M13 6l6 6-6 6"/></>,
    plus: <><path {...p} d="M12 5v14M5 12h14"/></>,
    spark: <><path {...p} d="M12 3v4M12 17v4M3 12h4M17 12h4M5.6 5.6l2.8 2.8M15.6 15.6l2.8 2.8M18.4 5.6l-2.8 2.8M8.4 15.6l-2.8 2.8"/></>,
    search: <><circle {...p} cx="11" cy="11" r="7"/><path {...p} d="m20 20-3.2-3.2"/></>,
    close: <><path {...p} d="M6 6l12 12M18 6 6 18"/></>,
    pause: <><path {...p} d="M9 5v14M15 5v14"/></>,
    stop: <><rect {...p} x="6" y="6" width="12" height="12" rx="2"/></>,
    ext: <><path {...p} d="M14 4h6v6M20 4l-9 9M18 13v5a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h5"/></>,
    check: <><path {...p} d="M4 12.5l5 5L20 6"/></>,
    share: <><circle {...p} cx="6" cy="12" r="2.6"/><circle {...p} cx="18" cy="6" r="2.6"/><circle {...p} cx="18" cy="18" r="2.6"/><path {...p} d="m8.3 10.8 7.4-3.6M8.3 13.2l7.4 3.6"/></>,
    clock: <><circle {...p} cx="12" cy="12" r="9"/><path {...p} d="M12 7v5l3.5 2"/></>,
    flame: <><path {...p} d="M12 3c1 3-2 4-2 7a2 2 0 0 0 4 0c0-1 0-1.5-.4-2.3C16 10 17 12.5 17 14.5a5 5 0 1 1-10 0C7 11 10 9 12 3Z"/></>,
    shield: <><path {...p} d="M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6l8-3Z"/></>,
    bolt2: <><path {...p} d="M11 3 5 13h5l-1 8 7-11h-5l1-7Z"/></>,
    chevR: <><path {...p} d="m9 6 6 6-6 6"/></>,
    x: <><path {...p} d="M4 4l16 16M20 4 4 20" strokeWidth="2"/></>,
    logo: <><polygon {...p} points="12,3.4 20.2,18.5 3.8,18.5"/><line {...p} x1="0.5" y1="13.7" x2="7.2" y2="13.7"/><line {...p} x1="14.4" y1="12.2" x2="23.5" y2="8.4"/><line {...p} x1="14.4" y1="13.3" x2="23.5" y2="11.7"/><line {...p} x1="14.4" y1="14.2" x2="23.5" y2="15.3"/><line {...p} x1="14.4" y1="15.1" x2="23.5" y2="18.7"/></>,
    copy: <><rect {...p} x="9" y="9" width="11" height="11" rx="2"/><path {...p} d="M5 15V5a2 2 0 0 1 2-2h8"/></>,
  };
  return (
    <svg viewBox="0 0 24 24" width={size} height={size} style={{ display: "block", ...style }}>
      {paths[name] || null}
    </svg>
  );
}

/* ---------- Prism mark (the brand logo) ----------
   White light enters the prism; the signal spectrum emerges.
   Outline + entry ray in ink; the four output rays are the
   fixed signal spectrum and are NEVER tinted by the accent. */
function PrismMark({ size = 26, ink = "var(--ink)", stroke = 3.4, style }) {
  return (
    <svg viewBox="0 0 100 100" width={size} height={size} fill="none" style={{ display: "block", ...style }} aria-label="Seer">
      <polygon points="50,14 84,77 16,77" stroke={ink} strokeWidth={stroke} strokeLinejoin="round" />
      <line x1="1" y1="57" x2="30" y2="57" stroke={ink} strokeWidth={stroke} strokeLinecap="round" />
      <line x1="60" y1="50" x2="99" y2="33" stroke="var(--c-alpha)" strokeWidth={stroke} strokeLinecap="round" />
      <line x1="60" y1="54" x2="99" y2="48" stroke="var(--c-opp)" strokeWidth={stroke} strokeLinecap="round" />
      <line x1="60" y1="58" x2="99" y2="63" stroke="var(--c-risk)" strokeWidth={stroke} strokeLinecap="round" />
      <line x1="60" y1="62" x2="99" y2="78" stroke="var(--c-anomaly)" strokeWidth={stroke} strokeLinecap="round" />
    </svg>
  );
}

/* ---------- Count-up number ---------- */
function CountUp({ to, dur = 1100, decimals = 0, prefix = "", suffix = "", className, style }) {
  const [v, setV] = useState(0);
  const started = useRef(false);
  useEffect(() => {
    if (started.current) { setV(to); return; }
    started.current = true;
    const t0 = performance.now();
    let raf;
    const tick = (t) => {
      const p = Math.min(1, (t - t0) / dur);
      const e = 1 - Math.pow(1 - p, 3);
      setV(to * e);
      if (p < 1) raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [to]);
  const out = v.toLocaleString(undefined, { minimumFractionDigits: decimals, maximumFractionDigits: decimals });
  return <span className={className} style={style}>{prefix}{out}{suffix}</span>;
}

/* ---------- Confidence bar ---------- */
function ConfidenceBar({ value, color = "var(--coral)", showPct = true, delay = 0 }) {
  const ref = useRef(null);
  useEffect(() => {
    const el = ref.current; if (!el) return;
    const id = setTimeout(() => { el.style.width = value + "%"; }, 60 + delay);
    return () => clearTimeout(id);
  }, [value]);
  return (
    <div className="row gap-8" style={{ width: "100%" }}>
      <div className="cbar grow"><i ref={ref} style={{ background: color }} /></div>
      {showPct && <span className="num" style={{ fontSize: 12, color: "var(--ink-2)", minWidth: 32, textAlign: "right" }}>{value}%</span>}
    </div>
  );
}

/* ---------- Category badge ---------- */
const CAT_STYLE = {
  ALPHA:       { c: "var(--c-alpha)", bg: "var(--c-alpha-wash)" },
  OPPORTUNITY: { c: "var(--c-opp)", bg: "var(--c-opp-wash)" },
  ANOMALY:     { c: "var(--c-anomaly)", bg: "var(--c-anomaly-wash)" },
  RISK:        { c: "var(--c-risk)", bg: "var(--c-risk-wash)" },
};
function CategoryBadge({ cat }) {
  const s = CAT_STYLE[cat] || CAT_STYLE.ALPHA;
  return <span className="badge" style={{ color: s.c, background: s.bg, borderColor: "transparent" }}>{cat}</span>;
}

/* ---------- Protocol badge ---------- */
function ProtocolBadge({ proto }) {
  if (!proto) return null;
  return (
    <span className="pill mono" style={{ fontSize: 11.5 }}>
      <span style={{ color: "var(--coral)", fontSize: 13 }}>{proto.glyph}</span>{proto.name}
    </span>
  );
}

/* ---------- Risk gauge (animated arc) ---------- */
function RiskGauge({ score, size = 132 }) {
  const [v, setV] = useState(0);
  useEffect(() => { const id = setTimeout(() => setV(score), 120); return () => clearTimeout(id); }, [score]);
  const r = size / 2 - 12;
  const cx = size / 2, cy = size / 2;
  const a0 = Math.PI * 0.75, a1 = Math.PI * 2.25; // 270deg arc
  const frac = v / 100;
  const aEnd = a0 + (a1 - a0) * frac;
  const pt = (a, rr = r) => [cx + Math.cos(a) * rr, cy + Math.sin(a) * rr];
  const arc = (from, to, rr = r) => {
    const [x0, y0] = pt(from, rr), [x1, y1] = pt(to, rr);
    const large = to - from > Math.PI ? 1 : 0;
    return `M ${x0} ${y0} A ${rr} ${rr} 0 ${large} 1 ${x1} ${y1}`;
  };
  const color = v < 40 ? "var(--safe)" : v < 70 ? "var(--warn)" : "var(--danger)";
  const label = v < 40 ? "Healthy" : v < 70 ? "Caution" : "Elevated";
  return (
    <div style={{ width: size, height: size * 0.78, position: "relative" }}>
      <svg width={size} height={size * 0.9} style={{ overflow: "visible" }}>
        <path d={arc(a0, a1)} stroke="var(--track)" strokeWidth="9" fill="none" strokeLinecap="round" />
        <path d={arc(a0, aEnd)} stroke={color} strokeWidth="9" fill="none" strokeLinecap="round"
              style={{ transition: "stroke 0.6s var(--ease), d 0.8s var(--ease)" }} />
      </svg>
      <div style={{ position: "absolute", inset: 0, top: 6, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center" }}>
        <span className="num" style={{ fontSize: 34, fontWeight: 600, color: "var(--ink)", lineHeight: 1 }}>
          <CountUp to={score} dur={900} />
        </span>
        <span style={{ fontSize: 11.5, color, fontWeight: 600, marginTop: 4, letterSpacing: "0.02em" }}>{label}</span>
      </div>
    </div>
  );
}

/* ---------- Line chart (you vs benchmark) ---------- */
function LineChart({ a, b, w = 520, h = 180, labelA = "You", labelB = "Smart money" }) {
  const all = [...a, ...b];
  const min = Math.min(...all), max = Math.max(...all);
  const nx = (i, arr) => (i / (arr.length - 1)) * w;
  const ny = (v) => h - ((v - min) / (max - min || 1)) * (h - 12) - 6;
  const path = (arr) => arr.map((v, i) => `${i ? "L" : "M"} ${nx(i, arr).toFixed(1)} ${ny(v).toFixed(1)}`).join(" ");
  const [drawn, setDrawn] = useState(false);
  useEffect(() => { const id = setTimeout(() => setDrawn(true), 80); return () => clearTimeout(id); }, []);
  return (
    <div>
      <svg viewBox={`0 0 ${w} ${h}`} width="100%" height={h} style={{ overflow: "visible" }}>
        {[0.25, 0.5, 0.75].map((g) => (
          <line key={g} x1="0" x2={w} y1={h * g} y2={h * g} stroke="var(--line-soft)" strokeWidth="1" />
        ))}
        <path d={`${path(b)} L ${w} ${h} L 0 ${h} Z`} fill="var(--sky-wash)" opacity={drawn ? 1 : 0} style={{ transition: "opacity .8s ease" }} />
        <path d={path(b)} fill="none" stroke="var(--sky)" strokeWidth="2" strokeDasharray="4 4"
              style={{ strokeDashoffset: drawn ? 0 : 1200, strokeDasharray: drawn ? "4 4" : "1200", transition: "stroke-dashoffset 1.2s var(--ease)" }} />
        <path d={path(a)} fill="none" stroke="var(--coral)" strokeWidth="2.5"
              pathLength="1" style={{ strokeDasharray: 1, strokeDashoffset: drawn ? 0 : 1, transition: "stroke-dashoffset 1.4s var(--ease)" }} />
      </svg>
      <div className="row gap-16" style={{ marginTop: 8 }}>
        <span className="row gap-6" style={{ fontSize: 12, color: "var(--ink-2)" }}><i style={{ width: 14, height: 2.5, background: "var(--coral)", borderRadius: 2 }} />{labelA}</span>
        <span className="row gap-6" style={{ fontSize: 12, color: "var(--ink-2)" }}><i style={{ width: 14, height: 0, borderTop: "2px dashed var(--sky)" }} />{labelB}</span>
      </div>
    </div>
  );
}

/* ---------- Empty state ---------- */
function EmptyState({ icon = "eye", title, body, cta, onCta }) {
  return (
    <div className="center" style={{ flexDirection: "column", textAlign: "center", padding: "64px 24px", gap: 14 }}>
      <div className="center" style={{ width: 64, height: 64, borderRadius: 99, background: "var(--coral-wash)", color: "var(--coral)", position: "relative" }}>
        <span className="dot live" style={{ position: "absolute", inset: 0, margin: "auto", width: 0, height: 0 }} />
        <Icon name={icon} size={26} />
      </div>
      <div className="serif" style={{ fontSize: 22, color: "var(--ink)" }}>{title}</div>
      {body && <div className="mut" style={{ maxWidth: 360, fontSize: 14 }}>{body}</div>}
      {cta && <button className="btn btn-primary" style={{ marginTop: 6 }} onClick={onCta}>{cta}<Icon name="arrow" size={16} /></button>}
    </div>
  );
}

/* ---------- Skeleton line ---------- */
function Skel({ w = "100%", h = 12, r = 6, style }) {
  return <div className="skel" style={{ width: w, height: h, borderRadius: r, ...style }} />;
}

/* ---------- relative time ---------- */
function relTime(ts) {
  const s = Math.floor((Date.now() - ts) / 1000);
  if (s < 60) return s <= 3 ? "just now" : s + "s ago";
  const m = Math.floor(s / 60); if (m < 60) return m + " min ago";
  const h = Math.floor(m / 60); if (h < 24) return h + "h ago";
  return Math.floor(h / 24) + "d ago";
}
function countdown(ts) {
  let s = Math.max(0, Math.floor((ts - Date.now()) / 1000));
  const d = Math.floor(s / 86400); s -= d * 86400;
  const h = Math.floor(s / 3600); s -= h * 3600;
  const m = Math.floor(s / 60); s -= m * 60;
  if (d > 0) return `${d}d ${h}h ${m}m`;
  return `${h}h ${m}m ${String(s).padStart(2, "0")}s`;
}

Object.assign(window, {
  Icon, PrismMark, CountUp, ConfidenceBar, CategoryBadge, ProtocolBadge,
  RiskGauge, LineChart, EmptyState, Skel, relTime, countdown, CAT_STYLE,
});
