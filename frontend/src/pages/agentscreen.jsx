/* ============================================================
   SEER - My Agent screen: chat state machine + intents rail
   ============================================================ */
import { useState, useRef, useEffect } from 'react';
import { sendOnChainTx } from '../utils/onchain.js';
import { SeerOrb } from './agent.jsx';

const SEER_GREETING = {
  id: "m-greet", role: "seer",
  text: ["I've read your wallet from the backend.",
         "Tell me what you want to happen - in plain words - and I'll ask Seer to turn it into a deployable plan."],
};

let msgSeq = 0;
const mid = () => "m" + (++msgSeq) + "-" + Date.now();

export function AgentScreen({ showToast }) {
  const seer = window.useSeerStore();
  const [messages, setMessages] = useState([SEER_GREETING]);
  const [input, setInput] = useState("");
  const [busy, setBusy] = useState(false);
  const [trace, setTrace] = useState(null);
  const scrollRef = useRef(null);
  const timers = useRef([]);
  const intents = seer.ACTIVE_INTENTS;

  // autoscroll to newest
  useEffect(() => {
    const el = scrollRef.current; if (!el) return;
    el.scrollTo({ top: el.scrollHeight, behavior: "smooth" });
  }, [messages]);
  useEffect(() => () => timers.current.forEach(clearTimeout), []);

  const push = (m) => setMessages((p) => [...p, { id: mid(), anim: true, ...m }]);
  const wait = (ms) => new Promise((res) => { timers.current.push(setTimeout(res, ms)); });

  async function runFlow(userText) {
    setBusy(true);
    push({ role: "user", text: [userText] });
    push({ role: "seer", kind: "typing" });
    try {
      const card = await window.SeerAPI.previewIntent(userText);
      setMessages((p) => p.filter((m) => m.kind !== "typing"));
      push({ role: "seer", text: ["I parsed that through the backend. Review the plan before anything is armed."] });
      await wait(250);
      push({ role: "seer", kind: "card", card });
    } catch (err) {
      setMessages((p) => p.filter((m) => m.kind !== "typing"));
      push({ role: "seer", text: [err.message || "I couldn't preview that intent from the backend."] });
      showToast(err.message || "Intent preview failed.");
    } finally {
      setBusy(false);
    }
  }

  const send = (text) => {
    const t = (text || "").trim();
    if (!t || busy) return;
    setInput("");
    runFlow(t);
  };

  const deploy = async (msgId, card) => {
    setBusy(true);
    try {
      // 1. Persist to backend + get on-chain calldata - rail NOT updated yet
      const result = await window.SeerAPI.deployIntent(card);

      // 2. Sign the on-chain tx first - user must confirm before we show anything
      if (result.register_intent_calldata) {
        const hash = await sendOnChainTx(result.register_intent_calldata);
        // 3. Tx signed - now commit to the rail and mark card done
        window.SeerAPI.commitIntent(result.intent);
        setMessages((p) => p.map((m) => m.id === msgId ? { ...m, done: true } : m));
        showToast(`Intent anchored on-chain - tx: ${hash.slice(0, 10)}…`, 'success');
      } else {
        // Contract not configured - still commit, just without on-chain anchor
        window.SeerAPI.commitIntent(result.intent);
        setMessages((p) => p.map((m) => m.id === msgId ? { ...m, done: true } : m));
        showToast("Agent deployed - Seer is on it.", 'success');
      }

      const t1 = setTimeout(() => push({ role: "seer", text: ["It's live. I'll show each backend action in the trace. You can pause me whenever."] }), 500);
      timers.current.push(t1);
    } catch (err) {
      // If the user rejected the tx or it failed, the card stays un-deployed
      showToast(err.message || "Deploy failed - intent not added.", 'error');
    } finally {
      setBusy(false);
    }
  };

  const cancelConfirm = (msgId) => {
    setMessages((p) => p.filter((m) => m.id !== msgId));
    showToast("Cancelled - nothing deployed.");
    const t = setTimeout(() => push({ role: "seer", text: ["No problem - nothing deployed. Tell me what you'd like instead."] }), 320);
    timers.current.push(t);
  };

  const toggle = async (id) => {
    const current = intents.find((i) => i.id === id);
    if (!current) return;
    try {
      await window.SeerAPI.setIntentStatus(id, current.status === "RUNNING" ? "PAUSED" : "RUNNING");
    } catch (err) {
      showToast(err.message || "Status update failed.");
    }
  };

  const running = intents.filter((i) => i.status === "RUNNING");
  const chips = window.SEER.TEMPLATES.map((t, i) => ({ k: String(i), label: t.label, text: t.text }));

  useEffect(() => {
    if (!seer.pendingIntentText || busy) return;
    const text = seer.pendingIntentText;
    window.SEER.update({ pendingIntentText: "" });
    send(text);
  }, [seer.pendingIntentText, busy]);

  return (
    <div className="seer-agent-shell">
      {/* ---- conversation ---- */}
      <section className="seer-chat">
        <header className="seer-chat-head">
          <div className="row gap-12">
            <SeerOrb size={40} thinking={busy} />
            <div className="col" style={{ lineHeight: 1.25 }}>
              <span className="serif" style={{ fontSize: 19, fontWeight: 500 }}>Seer</span>
              <span className="row gap-6 faint" style={{ fontSize: 12 }}>
                <span className="dot live" style={{ width: 6, height: 6, background: busy ? "var(--c-risk)" : "var(--c-opp)" }} />
                {busy ? "reading the chain…" : "online · your autonomous agent"}
              </span>
            </div>
          </div>
          <span className="pill"><Icon name="shield" size={13} />Non-custodial</span>
        </header>

        <div className="seer-chat-scroll" ref={scrollRef}>
          <div className="seer-chat-inner">
            {messages.map((m) => {
              if (m.kind === "typing") return <TypingMessage key={m.id} />;
              if (m.kind === "card")
                return (
                  <div key={m.id} className="seer-msg from-seer" style={{ animation: "msgIn .4s var(--ease-out) both" }}>
                    <SeerOrb size={28} />
                    <ConfirmAgentCard card={m.card} done={m.done}
                      onContinue={() => deploy(m.id, m.card)}
                      onCancel={() => cancelConfirm(m.id)}
                      onEdit={() => showToast("In the live product, every field is editable inline.")} />
                  </div>
                );
              return (
                <TextMessage key={m.id} role={m.role} anim={m.anim}>
                  {m.text.map((line, i) => <p key={i} className="seer-line">{line}</p>)}
                  {m.showChips && (
                    <div className="seer-inline-chips">
                      {chips.map((c) => <button key={c.k} className="seer-chip-sug" onClick={() => send(c.text || c.label)}>{c.label}</button>)}
                    </div>
                  )}
                </TextMessage>
              );
            })}
            {messages.length === 1 && (
              <div className="seer-starter-chips">
                {chips.map((c) => (
                  <button key={c.k} className="seer-starter-chip" onClick={() => send(c.text || c.label)}>
                    <Icon name="bolt2" size={15} style={{ color: "var(--coral)" }} />{c.label}
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>

        <div className="seer-composer">
          <div className="seer-composer-box">
            <input
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={(e) => { if (e.key === "Enter") send(input); }}
              placeholder="Tell Seer what to do…  e.g. “buy 150 USDY of mETH every Tuesday”"
              disabled={busy}
            />
            <button className="seer-send" disabled={busy || !input.trim()} onClick={() => send(input)} aria-label="Send">
              <Icon name="arrow" size={18} />
            </button>
          </div>
          <span className="faint" style={{ fontSize: 11, textAlign: "center" }}>Seer executes on-chain on your behalf. Every decision is logged and reversible.</span>
        </div>
      </section>

      {/* ---- intents rail ---- */}
      <aside className="seer-intents-rail">
        <div className="seer-rail-header">
          <span className="serif" style={{ fontSize: 17 }}>Your agents</span>
          <span className="pill" style={{ flexShrink: 0 }}><span className="dot live" style={{ width: 6, height: 6 }} />{running.length} running</span>
        </div>
        <p className="faint" style={{ fontSize: 12, margin: "2px 0 8px" }}>Everything Seer is doing for you, live.</p>
        {intents.length === 0 ? (
          <EmptyState icon="agent" title="No agents yet." body="Start a conversation - Seer will build your first one." />
        ) : (
          <div className="col gap-10">
            {intents.map((i) => <RailIntent key={i.id} intent={i} onTrace={setTrace} onToggle={toggle} />)}
          </div>
        )}
      </aside>

      {trace && <TraceModal intent={trace} onClose={() => setTrace(null)} />}
    </div>
  );
}

/* ---- compact intent card for the rail ---- */
const RAIL_STATUS = {
  RUNNING: { c: "var(--c-opp)", bg: "var(--c-opp-wash)" },
  PAUSED: { c: "var(--c-risk)", bg: "var(--c-risk-wash)" },
};
function RailIntent({ intent, onTrace, onToggle }) {
  const s = RAIL_STATUS[intent.status] || RAIL_STATUS.RUNNING;
  return (
    <div className="seer-rail-intent">
      <div className="row" style={{ justifyContent: "space-between", marginBottom: 8 }}>
        <span className="badge" style={{ color: s.c, background: s.bg }}>
          {intent.status === "RUNNING" && <span className="dot live" style={{ width: 5, height: 5 }} />}{intent.status}
        </span>
        {intent.pnl !== 0 && (
          <span className="num" style={{ fontSize: 12, color: intent.pnl >= 0 ? "var(--c-opp)" : "var(--danger)" }}>
            {intent.pnl >= 0 ? "+" : ""}${Math.abs(intent.pnl).toFixed(2)}
          </span>
        )}
      </div>
      <div style={{ fontSize: 13, fontWeight: 500, lineHeight: 1.35, marginBottom: 6 }}>{intent.summary}</div>
      <div className="faint" style={{ fontSize: 11.5, marginBottom: 8 }}>{intent.lastAction} · {relTime(intent.lastTs)}</div>
      <div className="seer-simulation-badge" style={{ marginBottom: 10, display: 'inline-block' }}>Simulation - not live</div>
      <div className="row gap-6">
        <button className="seer-rail-btn" onClick={() => onTrace(intent)}>Trace</button>
        <button className="seer-rail-btn" onClick={() => onToggle(intent.id)}>{intent.status === "RUNNING" ? "Pause" : "Resume"}</button>
      </div>
    </div>
  );
}

/* ---- reasoning trace modal (shared) ---- */
function TraceModal({ intent, onClose }) {
  return (
    <div className="seer-modal-bg" onClick={onClose}>
      <div className="card seer-trace-modal" onClick={(e) => e.stopPropagation()}>
        <div className="seer-trace-head">
          <div className="col" style={{ gap: 3 }}>
            <span className="eyebrow">Reasoning trace · on-chain log</span>
            <span className="serif" style={{ fontSize: 19 }}>{intent.summary}</span>
          </div>
          <button className="btn-quiet" style={{ padding: 6 }} onClick={onClose}><Icon name="close" size={18} /></button>
        </div>
        <div className="seer-trace-body">
          {(!intent.trace || intent.trace.length === 0) ? (
            <EmptyState icon="clock" title="Seer has not acted yet." body="Your first decision log will appear here. Next scheduled check in under an hour." />
          ) : (
            <div className="seer-timeline">
              {intent.trace.map((tr, i) => (
                <div key={i} className="seer-trace-item">
                  <span className="seer-trace-node" />
                  <div className="row gap-8" style={{ marginBottom: 5, flexWrap: "wrap" }}>
                    <span className="mono faint" style={{ fontSize: 11.5 }}>{tr.t}</span>
                    <span className="badge" style={{ color: "var(--coral-ink)", background: "var(--coral-wash)" }}>{tr.kind}</span>
                  </div>
                  <p className="mut" style={{ fontSize: 13, lineHeight: 1.55, margin: 0 }}>{tr.body}</p>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
