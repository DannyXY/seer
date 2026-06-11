/* ============================================================
   SEER - My Agent screen: chat state machine + intents rail
   ============================================================ */
import { useState, useRef, useEffect } from 'react';
import { sendOnChainTx } from '../utils/onchain.js';
import { ConfirmAgentCard, SeerOrb } from './agent.jsx';

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
  const [revokingApproval, setRevokingApproval] = useState(null);
  const scrollRef = useRef(null);
  const timers = useRef([]);
  const intents = seer.ACTIVE_INTENTS;
  const approvals = seer.APPROVALS || [];

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
      push({ role: "seer", text: ["I parsed that through the backend. The card shows what can actually be signed now versus what is anchor-only simulation."] });
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

  // One button drives the whole flow: anchor the intent on-chain and, when
  // executable testnet calldata exists, run it too. The wallet prompts for
  // each signature in sequence - the user only clicks once.
  const deploy = async (msgId, card) => {
    setBusy(true);
    const canExecute = !!card.capability?.canExecute;
    try {
      // 1. Persist to backend + get on-chain calldata - rail NOT updated yet
      const result = await window.SeerAPI.deployIntent(card);

      // 2. Anchor signature - user must confirm before we show anything
      let anchorHash = null;
      if (result.register_intent_calldata) {
        if (canExecute) showToast("Signature 1 of 2 - anchoring your intent on-chain.", 'info');
        anchorHash = await sendOnChainTx(result.register_intent_calldata);
      }

      // 3. Execution signature for the testnet draft, same click
      let execHash = null;
      let execError = null;
      if (canExecute) {
        try {
          if (anchorHash) showToast("Signature 2 of 2 - running the testnet transaction.", 'info');
          const { txHash } = await window.SeerAPI.signPreviewDraft(card);
          execHash = txHash;
        } catch (err) {
          execError = err; // anchor already succeeded - keep the intent, report the miss
        }
      }

      window.SeerAPI.commitIntent({
        ...result.intent,
        executionMode: anchorHash ? "anchor_only" : "simulation_only",
        anchorTxHash: anchorHash,
        lastAction: execHash && anchorHash
          ? "Anchored and executed on testnet"
          : execHash
            ? "Executed on testnet; no on-chain anchor configured"
            : anchorHash
              ? "Anchored on-chain; execution is simulation-only"
              : "Simulation-only; no on-chain anchor configured",
      });
      setMessages((p) => p.map((m) => m.id === msgId ? { ...m, done: true, actionDone: !!execHash } : m));

      if (execError) {
        showToast("Intent anchored, but the testnet transaction was not signed: " + (execError.message || "rejected"), 'error', anchorHash);
      } else if (execHash) {
        showToast("Intent anchored and testnet transaction submitted.", 'success', execHash);
      } else if (anchorHash) {
        showToast("Intent anchored on-chain for tracking.", 'success', anchorHash);
      } else {
        showToast("Saved as simulation-only.", 'success');
      }

      const followUp = execHash
        ? "Intent anchored and the testnet transaction was submitted from your wallet. Check approvals again after it confirms."
        : "Intent tracking is armed. I’ll show simulated decisions in the trace; autonomous live execution is not enabled in this build.";
      const t1 = setTimeout(() => push({ role: "seer", text: [followUp] }), 500);
      timers.current.push(t1);
    } catch (err) {
      // If the user rejected the tx or it failed, the card stays un-deployed
      showToast(err.message || "Deploy failed - intent not added.", 'error');
    } finally {
      setBusy(false);
    }
  };

  const revokeApproval = async (approval) => {
    if (!approval?.revoke_calldata || revokingApproval) return;
    setRevokingApproval(approval.id);
    try {
      const { txHash } = await window.SeerAPI.revokeApproval(approval);
      showToast(`Revoking ${approval.token_symbol} approval for ${approval.spender_label}.`, 'success', txHash);
    } catch (err) {
      showToast(err.message || "Approval revoke failed.", 'error');
    } finally {
      setRevokingApproval(null);
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
                      actionDone={m.actionDone}
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
          <span className="faint" style={{ fontSize: 11, textAlign: "center" }}>Seer drafts and simulates. You sign any on-chain transaction from your wallet.</span>
        </div>
      </section>

      {/* ---- intents rail ---- */}
      <aside className="seer-intents-rail">
        <div className="seer-rail-header">
          <span className="serif" style={{ fontSize: 17 }}>Your agents</span>
          <span className="pill" style={{ flexShrink: 0 }}><span className="dot live" style={{ width: 6, height: 6 }} />{running.length} running</span>
        </div>
        <p className="faint" style={{ fontSize: 12, margin: "2px 0 8px" }}>Anchored intents, simulations, and your testnet approvals.</p>
        <ApprovalsPanel approvals={approvals} onRevoke={revokeApproval} revokingApproval={revokingApproval} />
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

function ApprovalsPanel({ approvals, onRevoke, revokingApproval }) {
  const active = (approvals || []).filter((approval) => approval.active);
  const errors = (approvals || []).filter((approval) => approval.read_error);
  return (
    <div className="seer-approvals-panel">
      <div className="seer-approvals-head">
        <span className="eyebrow">Approvals</span>
        <span className="pill mono" style={{ fontSize: 10.5 }}>{active.length} active</span>
      </div>
      {active.length === 0 ? (
        <div className="mut" style={{ fontSize: 12.5, lineHeight: 1.45 }}>No active Seer testnet approvals.</div>
      ) : (
        <div className="col gap-8">
          {active.map((approval) => (
            <div key={approval.id} className="seer-approval-row">
              <div className="col" style={{ minWidth: 0, gap: 2 }}>
                <span className="row gap-6" style={{ fontSize: 12.5, fontWeight: 600 }}>
                  <span className="center seer-approval-token">{approval.token_symbol[0]}</span>
                  {approval.token_symbol}
                </span>
                <span className="faint" style={{ fontSize: 11 }}>{approval.spender_label}</span>
                <span className="mono mut" style={{ fontSize: 10.5 }}>{window.SEER.util.shortAddr(approval.spender_address)}</span>
              </div>
              <div className="col" style={{ alignItems: "flex-end", gap: 6 }}>
                <span className="num" style={{ fontSize: 12 }}>{approval.allowance_display}</span>
                <button
                  className="seer-approval-revoke"
                  disabled={!approval.revoke_calldata || revokingApproval === approval.id}
                  onClick={() => onRevoke(approval)}
                >
                  {revokingApproval === approval.id ? "Opening..." : "Revoke"}
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
      {errors.length > 0 && (
        <div className="seer-approval-error">{errors.length} approval check{errors.length === 1 ? "" : "s"} could not be read.</div>
      )}
    </div>
  );
}

/* ---- compact intent card for the rail ---- */
const RAIL_STATUS = {
  RUNNING: { c: "var(--c-opp)", bg: "var(--c-opp-wash)" },
  PAUSED: { c: "var(--c-risk)", bg: "var(--c-risk-wash)" },
};
function railMode(intent) {
  if (intent.executionMode === "anchor_only") return { label: "Anchor only", className: "anchor" };
  return { label: "Simulation only", className: "" };
}
function anchorLink(intent) {
  const contracts = window.SEER?.CONTRACTS;
  if (intent.anchorTxHash && contracts?.explorer_base) {
    return { href: `${contracts.explorer_base}/tx/${intent.anchorTxHash}`, label: "Anchor tx · verify" };
  }
  if (intent.onchainIntentId && contracts?.intent_registry) {
    return { href: `${contracts.explorer_base}/address/${contracts.intent_registry}`, label: `Anchored on-chain #${intent.onchainIntentId} · verify` };
  }
  return null;
}

function RailIntent({ intent, onTrace, onToggle }) {
  const s = RAIL_STATUS[intent.status] || RAIL_STATUS.RUNNING;
  const mode = railMode(intent);
  const pnl = Number(intent.pnl);
  const hasPnl = Number.isFinite(pnl) && pnl !== 0;
  const verify = anchorLink(intent);
  return (
    <div className="seer-rail-intent">
      <div className="row" style={{ justifyContent: "space-between", marginBottom: 8 }}>
        <span className="badge" style={{ color: s.c, background: s.bg }}>
          {intent.status === "RUNNING" && <span className="dot live" style={{ width: 5, height: 5 }} />}{intent.status}
        </span>
        {hasPnl && (
          <span className="num" style={{ fontSize: 12, color: pnl >= 0 ? "var(--c-opp)" : "var(--danger)" }}>
            {pnl >= 0 ? "+" : ""}${Math.abs(pnl).toFixed(2)}
          </span>
        )}
      </div>
      <div style={{ fontSize: 13, fontWeight: 500, lineHeight: 1.35, marginBottom: 6 }}>{intent.summary}</div>
      <div className="faint" style={{ fontSize: 11.5, marginBottom: 8 }}>{intent.lastAction} · {relTime(intent.lastTs)}</div>
      <div className={"seer-simulation-badge " + mode.className} style={{ marginBottom: 10, display: 'inline-block' }}>{mode.label}</div>
      {verify && (
        <a
          className="row gap-5 mono"
          style={{ fontSize: 10.5, color: "var(--ink-2)", textDecoration: "none", marginBottom: 10 }}
          href={verify.href} target="_blank" rel="noopener noreferrer"
        >
          <Icon name="shield" size={10} />{verify.label}<Icon name="arrow-up-right" size={9} />
        </a>
      )}
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
