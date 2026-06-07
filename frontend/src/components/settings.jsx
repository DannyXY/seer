/* ============================================================
   SEER — Settings
   ============================================================ */
import { useState, useEffect } from 'react';

function Toggle({ on, onChange }) {
  return (
    <button className={"seer-toggle" + (on ? " on" : "")} onClick={() => onChange(!on)} role="switch" aria-checked={on}>
      <span className="seer-toggle-knob" />
    </button>
  );
}

function SettingRow({ title, desc, children }) {
  return (
    <div className="seer-set-row">
      <div className="col" style={{ gap: 3, minWidth: 0 }}>
        <span style={{ fontSize: 14, fontWeight: 500 }}>{title}</span>
        {desc && <span className="mut" style={{ fontSize: 12.5, lineHeight: 1.45 }}>{desc}</span>}
      </div>
      <div style={{ flexShrink: 0 }}>{children}</div>
    </div>
  );
}

export function SettingsScreen() {
  const seer = window.useSeerStore();
  const settings = seer.settings;
  const identity = seer.IDENTITY;
  const save = (patch) => window.SeerAPI.saveSettings(patch);

  return (
    <div className="seer-screen">
      <header className="seer-screen-head">
        <div className="col" style={{ gap: 9 }}>
          <h1 className="serif seer-h1">Settings</h1>
          <p className="seer-screen-sub" style={{ margin: 0 }}>Tune how Seer notifies you, what your agent is allowed to do, and how your identity is held.</p>
        </div>
      </header>

      <div className="col gap-24">
        {/* Notifications */}
        <section className="card seer-set-block">
          <div className="seer-set-head"><span className="center seer-set-ic"><Icon name="signal" size={16} /></span><span className="serif" style={{ fontSize: 18 }}>Notifications</span></div>
          <SettingRow title="Telegram alerts" desc="Seer messages you when something needs your eyes.">
            <Toggle on={settings.telegramAlerts} onChange={(v) => save({ telegramAlerts: v })} />
          </SettingRow>
          <div className="seer-set-div" />
          <SettingRow title="Risk score threshold" desc={`Alert when portfolio risk crosses ${settings.riskAlert}.`}>
            <div className="row gap-10"><input type="range" min="40" max="90" value={settings.riskAlert} onChange={(e) => save({ riskAlert: +e.target.value })} className="seer-range" style={{ width: 130 }} /><span className="num" style={{ width: 28 }}>{settings.riskAlert}</span></div>
          </SettingRow>
          <div className="seer-set-div" />
          <SettingRow title="Signal confidence threshold" desc={`Only surface signals above ${settings.confidenceAlert}% confidence.`}>
            <div className="row gap-10"><input type="range" min="50" max="95" value={settings.confidenceAlert} onChange={(e) => save({ confidenceAlert: +e.target.value })} className="seer-range" style={{ width: 130 }} /><span className="num" style={{ width: 36 }}>{settings.confidenceAlert}%</span></div>
          </SettingRow>
          <div className="seer-set-div" />
          <SettingRow title="Depeg sensitivity" desc={`Flag stablecoin deviations beyond ${settings.depegSensitivity}%.`}>
            <div className="row gap-10"><input type="range" min="1" max="5" step="0.5" value={settings.depegSensitivity} onChange={(e) => save({ depegSensitivity: +e.target.value })} className="seer-range" style={{ width: 130 }} /><span className="num" style={{ width: 36 }}>{settings.depegSensitivity}%</span></div>
          </SettingRow>
        </section>

        {/* Agent permissions */}
        <section className="card seer-set-block">
          <div className="seer-set-head"><span className="center seer-set-ic"><Icon name="shield" size={16} /></span><span className="serif" style={{ fontSize: 18 }}>Agent permissions</span></div>
          <SettingRow title="Autonomous execution" desc="Allow the agent to act without per-trade confirmation.">
            <Toggle on={settings.autonomousExecution} onChange={(v) => save({ autonomousExecution: v })} />
          </SettingRow>
          <div className="seer-set-div" />
          <SettingRow title="Spending limit" desc="Maximum the agent can deploy per intent.">
            <div className="row gap-10"><input type="range" min="500" max="10000" step="500" value={settings.spendLimit} onChange={(e) => save({ spendLimit: +e.target.value })} className="seer-range" style={{ width: 130 }} /><span className="num" style={{ width: 64 }}>${settings.spendLimit.toLocaleString()}</span></div>
          </SettingRow>
          <div className="seer-set-div" />
          <div className="seer-set-row">
            <div className="col" style={{ gap: 3 }}>
              <span style={{ fontSize: 14, fontWeight: 500 }}>Authorized contracts</span>
              <span className="mut" style={{ fontSize: 12.5 }}>Agni Finance · INIT Capital · Lendle · Merchant Moe</span>
            </div>
            <button className="btn btn-ghost" style={{ padding: "8px 12px", fontSize: 13, color: "var(--danger)", borderColor: "var(--coral-line)" }}>Revoke all</button>
          </div>
        </section>

        {/* Identity */}
        <section className="card seer-set-block">
          <div className="seer-set-head"><span className="center seer-set-ic"><Icon name="identity" size={16} /></span><span className="serif" style={{ fontSize: 18 }}>Identity</span></div>
          <SettingRow title="Soulbound token" desc="Your identity is minted as a non-transferable SBT.">
            <span className="pill"><span className="dot" style={{ background: identity.sbt.minted ? "var(--c-opp)" : "var(--ink-3s)" }} />{identity.sbt.minted ? `Token #${identity.sbt.token}` : "Not minted"}</span>
          </SettingRow>
          <div className="seer-set-div" />
          <SettingRow title="Archetype" desc="Recalculated from on-chain behavior. Updatable once every 30 days.">
            <button className="btn btn-ghost" style={{ padding: "8px 12px", fontSize: 13 }} onClick={() => window.SeerAPI.bootstrap(seer.wallet)}>Re-read identity</button>
          </SettingRow>
        </section>
      </div>
    </div>
  );
}
