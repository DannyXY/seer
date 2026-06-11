/* ============================================================
   SEER - My Agent: the conversation is the product
   Chat-led intent creation à la Magic Newton. Seer has a voice,
   a face (the orb), and surfaces structured "Confirm Agent" cards.
   ============================================================ */

/* ---------- Seer's living orb avatar ---------- */
export function SeerOrb({ size = 30, thinking = false }) {
  return (
    <span className={"seer-orb" + (thinking ? " thinking" : "")} style={{ width: size, height: size }}>
      <span className="seer-orb-core" />
      <span className="seer-orb-ring" />
    </span>
  );
}

/* ---------- message bubbles ---------- */
function TextMessage({ role, children, anim }) {
  const seer = role === "seer";
  return (
    <div className={"seer-msg " + (seer ? "from-seer" : "from-user")} style={anim ? { animation: "msgIn .4s var(--ease-out) both" } : null}>
      {seer && <SeerOrb size={28} />}
      <div className="seer-bubble">{children}</div>
    </div>
  );
}

function TypingMessage() {
  return (
    <div className="seer-msg from-seer" style={{ animation: "msgIn .3s var(--ease-out) both" }}>
      <SeerOrb size={28} thinking />
      <div className="seer-bubble seer-typing">
        <span className="seer-typing-label">Seer is reading the chain</span>
        <span className="seer-dots"><i /><i /><i /></span>
      </div>
    </div>
  );
}

/* ---------- the confirm-agent card (Magic-Newton style) ---------- */
const TOKEN_GLYPH = { USDY: "$", mETH: "Ξ", MNT: "M", USDC: "$", Guardrail: "◈", Mirror: "◎" };
function capabilityClass(tone) {
  if (tone === "ready") return " ready";
  if (tone === "danger") return " danger";
  return "";
}

export function ConfirmAgentCard({ card, onContinue, onEdit, onCancel, done, actionDone }) {
  const cs = CAT_STYLE[card.accent] || CAT_STYLE.ALPHA;
  const capability = card.capability || {
    label: "Simulation only",
    tone: "warn",
    canExecute: false,
    body: "Seer can anchor and monitor this intent, but no executable calldata is available.",
  };
  return (
    <div className={"seer-confirm" + (done ? " done" : "")}>
      <div className="seer-confirm-top">
        <span className="seer-confirm-bar" style={{ background: cs.c }} />
        <span className="eyebrow" style={{ color: "var(--ink-2)" }}>{done ? "Intent anchored" : "Review capability"}</span>
      </div>
      <div className="seer-confirm-title serif">{card.title}</div>
      <div className="seer-confirm-rows">
        {card.rows.map((r) => (
          <div key={r.k} className="seer-confirm-row">
            <span className="seer-confirm-k">{r.k}</span>
            <span className="seer-confirm-v num">
              {r.token && <span className="seer-tok-mini" style={{ color: cs.c, borderColor: "var(--line)" }}>{TOKEN_GLYPH[r.token] || "◇"}</span>}
              {r.v}
            </span>
          </div>
        ))}
      </div>
      <div className="seer-confirm-chip">
        <span className="center seer-confirm-chip-ic" style={{ background: cs.bg, color: cs.c }}>{TOKEN_GLYPH[card.chip.sym] || "◇"}</span>
        <div className="col" style={{ lineHeight: 1.3, minWidth: 0 }}>
          <span style={{ fontWeight: 600, fontSize: 13 }}>{card.chip.sym}</span>
          <span className="faint" style={{ fontSize: 11.5 }}>{card.chip.note}</span>
        </div>
      </div>
      <div className="seer-simulation-banner">
        <span className={"seer-simulation-badge" + capabilityClass(capability.tone)}>{capability.label}</span>
        <span>{capability.body}</span>
      </div>
      {actionDone && (
        <div className="seer-confirm-done"><Icon name="check" size={15} style={{ color: "var(--c-opp)" }} />Testnet transaction submitted</div>
      )}
      {done ? (
        <div className="seer-confirm-done"><Icon name="check" size={15} style={{ color: "var(--c-opp)" }} />Anchored for tracking</div>
      ) : (
        <div className="seer-confirm-actions">
          <button className="btn btn-ghost" style={{ justifyContent: "center" }} onClick={onCancel}>Cancel</button>
          <button className="btn btn-ghost" style={{ flex: 1, justifyContent: "center" }} onClick={onEdit}>Edit</button>
          <button className="btn btn-primary" style={{ flex: 2, justifyContent: "center" }} onClick={onContinue}>
            {capability.canExecute ? "Sign & deploy" : "Anchor intent"}<Icon name="arrow" size={15} />
          </button>
        </div>
      )}
    </div>
  );
}

window.SeerOrb = SeerOrb;
window.TextMessage = TextMessage;
window.TypingMessage = TypingMessage;
window.ConfirmAgentCard = ConfirmAgentCard;
