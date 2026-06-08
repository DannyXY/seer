/* ============================================================
   SEER — Identity card renderer (canvas → shareable image)
   Drawn on canvas so the on-screen card and the export are
   byte-identical. Geometry-only sigils, seeded per archetype.
   ============================================================ */

const ARCH_HEX = {
  strategist: { accent: "#4E9BFF", deep: "#2E6FBF", glyph: 7 },
  yieldvampire: { accent: "#E15CD2", deep: "#A23A95", glyph: 5 },
  diamondhand: { accent: "#2BD4BE", deep: "#1C8E80", glyph: 6 },
  contrarian: { accent: "#F7A833", deep: "#B87A1E", glyph: 9 },
  degen:      { accent: "#FF6B4A", deep: "#B84128", glyph: 8 },
};

/* deterministic PRNG */
function mulberry(seed) {
  return function () {
    seed |= 0; seed = (seed + 0x6D2B79F5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function drawSigil(ctx, cx, cy, R, accent, deep, n) {
  const rnd = mulberry(n * 1337 + 7);
  ctx.save();
  ctx.translate(cx, cy);
  // outer halo rings
  ctx.lineWidth = 1.2;
  for (let i = 0; i < 3; i++) {
    ctx.beginPath(); ctx.arc(0, 0, R * (0.62 + i * 0.18), 0, Math.PI * 2);
    ctx.strokeStyle = i === 1 ? accent : "rgba(255,255,255,0.16)";
    ctx.globalAlpha = i === 1 ? 0.55 : 0.4; ctx.stroke();
  }
  ctx.globalAlpha = 1;
  // n-gon star
  const pts = [];
  for (let i = 0; i < n; i++) {
    const a = (i / n) * Math.PI * 2 - Math.PI / 2;
    pts.push([Math.cos(a) * R * 0.6, Math.sin(a) * R * 0.6]);
  }
  ctx.lineWidth = 1.4; ctx.strokeStyle = accent; ctx.globalAlpha = 0.9;
  const step = n % 2 === 0 ? 1 : Math.floor(n / 2);
  ctx.beginPath();
  for (let i = 0; i <= n; i++) {
    const p = pts[(i * step) % n];
    if (i === 0) ctx.moveTo(p[0], p[1]); else ctx.lineTo(p[0], p[1]);
  }
  ctx.stroke();
  // radial spokes
  ctx.globalAlpha = 0.3; ctx.strokeStyle = deep; ctx.lineWidth = 1;
  for (let i = 0; i < n; i++) {
    const a = (i / n) * Math.PI * 2 - Math.PI / 2;
    ctx.beginPath(); ctx.moveTo(0, 0); ctx.lineTo(Math.cos(a) * R * 0.6, Math.sin(a) * R * 0.6); ctx.stroke();
  }
  // scattered nodes
  ctx.globalAlpha = 1;
  pts.forEach((p) => {
    ctx.beginPath(); ctx.arc(p[0], p[1], 3, 0, Math.PI * 2);
    ctx.fillStyle = accent; ctx.fill();
  });
  // inner eye
  ctx.beginPath(); ctx.arc(0, 0, R * 0.2, 0, Math.PI * 2);
  ctx.fillStyle = "rgba(10,11,13,0.92)"; ctx.fill();
  ctx.beginPath(); ctx.arc(0, 0, R * 0.09, 0, Math.PI * 2);
  ctx.fillStyle = accent; ctx.fill();
  // faint outer dots
  ctx.globalAlpha = 0.5;
  for (let i = 0; i < 24; i++) {
    const a = rnd() * Math.PI * 2, rr = R * (0.85 + rnd() * 0.12);
    ctx.beginPath(); ctx.arc(Math.cos(a) * rr, Math.sin(a) * rr, 1.1, 0, Math.PI * 2);
    ctx.fillStyle = deep; ctx.fill();
  }
  ctx.restore();
}

/* draw the whole card at logical 360x520, scaled by `scale` */
function drawIdentityCard(ctx, scale, archKey, data) {
  const W = 360, H = 520;
  const A = window.SEER.ARCHETYPES[archKey] || window.SEER.ARCHETYPES.strategist;
  const hx = ARCH_HEX[archKey] || ARCH_HEX.strategist;
  ctx.save();
  ctx.scale(scale, scale);
  ctx.clearRect(0, 0, W, H);

  // background graphite
  const g = ctx.createLinearGradient(0, 0, W, H);
  g.addColorStop(0, "#131519"); g.addColorStop(1, "#0A0B0D");
  ctx.fillStyle = g; ctx.fillRect(0, 0, W, H);
  // subtle vignette tint of accent
  const rg = ctx.createRadialGradient(W / 2, 188, 20, W / 2, 188, 220);
  rg.addColorStop(0, hx.accent + "24"); rg.addColorStop(1, "#00000000");
  ctx.fillStyle = rg; ctx.fillRect(0, 0, W, H);

  // double frame
  ctx.strokeStyle = "#262A31"; ctx.lineWidth = 1.5;
  ctx.strokeRect(14, 14, W - 28, H - 28);
  ctx.strokeStyle = hx.accent; ctx.globalAlpha = 0.6; ctx.lineWidth = 1;
  ctx.strokeRect(20, 20, W - 40, H - 40);
  ctx.globalAlpha = 1;

  const ink = "#EDEFF2", mut = "#8B919B";
  ctx.textAlign = "center";

  // top label
  ctx.fillStyle = mut; ctx.font = "500 11px 'JetBrains Mono', monospace";
  ctx.fillText("· T H E   S E E R ·", W / 2, 44);

  // corner roman numerals
  ctx.fillStyle = ink; ctx.font = "700 16px 'Archivo', sans-serif";
  ctx.textAlign = "left"; ctx.fillText(A.roman, 30, 44);
  ctx.textAlign = "right"; ctx.fillText(A.roman, W - 30, 44);
  ctx.textAlign = "center";

  // sigil
  drawSigil(ctx, W / 2, 150, 78, hx.accent, hx.deep, A.hueN || hx.glyph);

  // archetype name
  ctx.fillStyle = ink; ctx.font = "800 30px 'Archivo', sans-serif";
  ctx.fillText((A.name || "").toUpperCase(), W / 2, 280);
  // tagline
  ctx.fillStyle = hx.accent; ctx.font = "500 13px 'JetBrains Mono', monospace";
  ctx.fillText("“" + A.tagline + "”", W / 2, 304);

  // divider
  ctx.strokeStyle = "#262A31"; ctx.lineWidth = 1;
  ctx.beginPath(); ctx.moveTo(46, 322); ctx.lineTo(W - 46, 322); ctx.stroke();
  ctx.fillStyle = hx.accent; ctx.beginPath(); ctx.arc(W / 2, 322, 2.5, 0, Math.PI * 2); ctx.fill();

  // stats 2x2
  const stats = (data.stats || []).slice(0, 4);
  const colX = [W / 2 - 70, W / 2 + 70];
  const rowY = [354, 400];
  ctx.textAlign = "center";
  stats.forEach((s, i) => {
    const x = colX[i % 2], y = rowY[Math.floor(i / 2)];
    ctx.fillStyle = mut; ctx.font = "500 9px 'JetBrains Mono', monospace";
    ctx.fillText(s.k.toUpperCase(), x, y);
    ctx.fillStyle = ink; ctx.font = "700 18px 'JetBrains Mono', monospace";
    ctx.fillText(String(s.v), x, y + 20);
  });

  // percentile pill
  ctx.fillStyle = hx.accent + "22";
  const pillW = 200, pillX = (W - pillW) / 2, pillY = 432;
  roundRect(ctx, pillX, pillY, pillW, 26, 13); ctx.fill();
  ctx.fillStyle = hx.accent; ctx.font = "500 11px 'JetBrains Mono', monospace";
  ctx.fillText(data.percentileLabel, W / 2, pillY + 17);

  // SBT line
  ctx.fillStyle = mut; ctx.font = "400 11px 'JetBrains Mono', monospace";
  const sbt = data.sbt.minted ? "◆ SBT MINTED · TOKEN #" + data.sbt.token : "◇ NOT YET MINTED";
  ctx.fillText(sbt, W / 2, 478);
  // wallet
  ctx.fillStyle = "#5A606A"; ctx.font = "400 10px 'JetBrains Mono', monospace";
  ctx.fillText(window.SEER.util.shortAddr(data.wallet), W / 2, 496);

  ctx.restore();
}

function roundRect(ctx, x, y, w, h, r) {
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.arcTo(x + w, y, x + w, y + h, r);
  ctx.arcTo(x + w, y + h, x, y + h, r);
  ctx.arcTo(x, y + h, x, y, r);
  ctx.arcTo(x, y, x + w, y, r);
  ctx.closePath();
}

export { drawIdentityCard, ARCH_HEX };
