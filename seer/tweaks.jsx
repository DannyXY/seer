/* ============================================================
   SEER — Tweaks (accent / canvas / motion)  ·  "Instrument"
   Runtime source of truth for the brand tokens. Applied on load
   so the whole app + landing render in the Instrument system.
   ============================================================ */

/* Voltage is the signature; the spectrum hues are offered as
   alternates for exploration. All tuned to read on graphite. */
const SEER_ACCENTS = {
  "#C8F230": { name: "Voltage", c: "#C8F230", h: "#A6CE1E", ink: "#0C0E08", rgb: "200,242,48" },
  "#4E9BFF": { name: "Azure",   c: "#4E9BFF", h: "#3D86E8", ink: "#06121F", rgb: "78,155,255" },
  "#2BD4BE": { name: "Teal",    c: "#2BD4BE", h: "#20BAA6", ink: "#04130F", rgb: "43,212,190" },
  "#F7A833": { name: "Amber",   c: "#F7A833", h: "#E0901A", ink: "#1A0E00", rgb: "247,168,51" },
  "#E15CD2": { name: "Magenta", c: "#E15CD2", h: "#CE3FBE", ink: "#1A0617", rgb: "225,92,210" },
};

const SEER_CANVAS = {
  Graphite: { dark:1, paper:"#0A0B0D", sink:"#08090B", card:"#131519", card2:"#1A1D22", line:"#262A31", lineSoft:"#1B1E23", ink:"#EDEFF2", ink2:"#8B919B", ink3:"#5A606A", track:"#212429", skel1:"#131519", skel2:"#1A1D22", scroll:"#2b2f36", scrollH:"#3a3f47", overlay:"rgba(255,255,255,0.05)" },
  Onyx:     { dark:1, paper:"#0C0D11", sink:"#08090C", card:"#16181E", card2:"#1C1F27", line:"#2A2E38", lineSoft:"#1F232B", ink:"#F2F4F8", ink2:"#8E95A1", ink3:"#5C636F", track:"#23262F", skel1:"#16181E", skel2:"#1C1F27", scroll:"#2A2E38", scrollH:"#383D49", overlay:"rgba(255,255,255,0.055)" },
  Black:    { dark:1, paper:"#000000", sink:"#000000", card:"#0E0F12", card2:"#16171B", line:"#26282E", lineSoft:"#1B1C20", ink:"#F4F6FA", ink2:"#8C929C", ink3:"#585E68", track:"#1B1D22", skel1:"#0E0F12", skel2:"#16171B", scroll:"#26282E", scrollH:"#34373F", overlay:"rgba(255,255,255,0.06)" },
};

// the signal spectrum stays fixed regardless of the chosen accent —
// ALPHA is azure (a reading), never the voltage instrument color.
const SEER_ALPHA = { c: "#4E9BFF", wash: "rgba(78,155,255,0.14)" };

const SEER_TWEAK_DEFAULTS = /*EDITMODE-BEGIN*/{
  "accent": "#C8F230",
  "canvas": "Graphite",
  "motion": "Standard"
}/*EDITMODE-END*/;

function applySeerTheme(t) {
  const root = document.documentElement;
  const a = SEER_ACCENTS[t.accent] || SEER_ACCENTS["#C8F230"];
  const cv = SEER_CANVAS[t.canvas] || SEER_CANVAS.Graphite;

  root.style.setProperty("--volt", a.c);
  root.style.setProperty("--volt-ink", a.ink);
  root.style.setProperty("--coral", a.c);
  root.style.setProperty("--coral-hover", a.h);
  root.style.setProperty("--coral-ink", `color-mix(in srgb, ${a.c} 70%, white)`);
  root.style.setProperty("--coral-wash", `color-mix(in srgb, ${a.c} 14%, transparent)`);
  root.style.setProperty("--coral-line", `color-mix(in srgb, ${a.c} 34%, transparent)`);
  root.style.setProperty("--coral-glow", `0 0 0 1px ${`color-mix(in srgb, ${a.c} 30%, transparent)`}, 0 0 22px ${`color-mix(in srgb, ${a.c} 22%, transparent)`}`);
  root.style.setProperty("--coral-rgb", a.rgb);
  window.SEER_ACCENT = a.rgb;

  // ALPHA signal category is fixed azure (a reading, not the instrument)
  root.style.setProperty("--c-alpha", SEER_ALPHA.c);
  root.style.setProperty("--c-alpha-wash", SEER_ALPHA.wash);

  root.style.setProperty("--paper", cv.paper);
  root.style.setProperty("--paper-sink", cv.sink);
  root.style.setProperty("--card", cv.card);
  root.style.setProperty("--card-2", cv.card2);
  root.style.setProperty("--line", cv.line);
  root.style.setProperty("--line-soft", cv.lineSoft);
  root.style.setProperty("--ink", cv.ink);
  root.style.setProperty("--ink-2", cv.ink2);
  root.style.setProperty("--ink-3s", cv.ink3);
  root.style.setProperty("--ink-3", cv.ink3 + "88");
  root.style.setProperty("--track", cv.track);
  root.style.setProperty("--skel-1", cv.skel1);
  root.style.setProperty("--skel-2", cv.skel2);
  root.style.setProperty("--scroll", cv.scroll);
  root.style.setProperty("--scroll-hover", cv.scrollH);
  root.style.setProperty("--hover-overlay", cv.overlay);
  root.classList.toggle("seer-dark", !!cv.dark);

  root.style.setProperty("--motion", t.motion === "Calm" ? "0.6" : "1");
}

function SeerTweaks() {
  const [t, setTweak] = useTweaks(SEER_TWEAK_DEFAULTS);
  React.useEffect(() => { applySeerTheme(t); }, [t.accent, t.canvas, t.motion]);
  return (
    <TweaksPanel title="Tweaks">
      <TweakSection label="Voltage" />
      <TweakColor label="Accent" value={t.accent}
        options={Object.keys(SEER_ACCENTS)}
        onChange={(v) => setTweak("accent", v)} />
      <TweakSection label="Surface" />
      <TweakRadio label="Canvas" value={t.canvas} options={["Graphite", "Onyx", "Black"]}
        onChange={(v) => setTweak("canvas", v)} />
      <TweakSection label="Motion" />
      <TweakRadio label="Speed" value={t.motion} options={["Calm", "Standard"]}
        onChange={(v) => setTweak("motion", v)} />
    </TweaksPanel>
  );
}

// apply persisted theme immediately on load (before edit mode opened)
(function () {
  try {
    const saved = JSON.parse(localStorage.getItem("tweaks:" + location.pathname) || "null");
    applySeerTheme(Object.assign({}, SEER_TWEAK_DEFAULTS, saved || {}));
  } catch (e) { applySeerTheme(SEER_TWEAK_DEFAULTS); }
})();

window.SeerTweaks = SeerTweaks;
