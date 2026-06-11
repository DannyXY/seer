/* ============================================================
   SEER - app root + routing with Privy auth
   ============================================================ */
import React, { useState, useEffect, useCallback, useRef } from 'react';
import { usePrivy, useWallets } from '@privy-io/react-auth';
import Landing from './pages/landing';
import { SignalFeed } from './components/signalfeed';
import { AgentScreen } from './pages/agentscreen';
import { IdentityScreen } from './pages/identity';
import { ArenaScreen } from './pages/arena';
import { SettingsScreen } from './components/settings';
import { Sidebar, RightRail } from './components/shell';

function Placeholder({ title, sub }) {
  return (
    <div className="seer-screen">
      <header className="seer-screen-head"><div><h1 className="serif seer-h1">{title}</h1><p className="seer-screen-sub">{sub}</p></div></header>
      <div style={{ padding: '20px' }}>Coming together next.</div>
    </div>
  );
}

const ROUTE_PATHS = {
  feed: "/signals",
  agent: "/agents",
  identity: "/identity",
  arena: "/arena",
  settings: "/settings",
};
const PATH_ROUTES = Object.fromEntries(Object.entries(ROUTE_PATHS).map(([route, path]) => [path, route]));

function routeFromPath(pathname) {
  const clean = pathname.replace(/\/+$/, "") || "/";
  if (clean === "/" || clean === "/Seer.html" || clean === "/index.html") return "home";
  return PATH_ROUTES[clean] || "home";
}

function pushRoute(route, replace = false) {
  if (route === "home") {
    if (window.location.pathname === "/") return;
    const fn = replace ? "replaceState" : "pushState";
    window.history[fn]({ route }, "", "/");
    return;
  }
  const path = ROUTE_PATHS[route] || ROUTE_PATHS.agent;
  if (window.location.pathname === path) return;
  const fn = replace ? "replaceState" : "pushState";
  window.history[fn]({ route }, "", path);
}

const TOAST_ICONS = { success: "check", error: "x", info: "signal" };
const EXPLORER_TX = "https://explorer.sepolia.mantle.xyz/tx/";

function Toast({ toast }) {
  if (!toast) return null;
  const icon = TOAST_ICONS[toast.kind] || "signal";
  return (
    <div className={"seer-toast seer-toast--" + toast.kind}>
      <div className="seer-toast-icon"><Icon name={icon} size={14} /></div>
      <div className="seer-toast-body">
        <span className="seer-toast-msg">{toast.msg}</span>
        {toast.txHash && (
          <a
            className="seer-toast-tx"
            href={EXPLORER_TX + toast.txHash}
            target="_blank"
            rel="noopener noreferrer"
          >
            {toast.txHash.slice(0, 10)}…{toast.txHash.slice(-6)}
            <Icon name="arrow-up-right" size={11} />
          </a>
        )}
      </div>
    </div>
  );
}

function LoadingScreen({ message = "Seer is loading authenticated Mantle data from the backend." }) {
  return (
    <div className="seer-loading-screen">
      <div className="serif seer-loading-title">Reading your wallet.</div>
      <div className="mut seer-loading-copy">{message}</div>
      <div className="seer-loading-bar"><i /></div>
    </div>
  );
}

function ConnectIssueScreen({ title, message, onRetry, onLogout }) {
  return (
    <div className="seer-loading-screen">
      <div className="serif seer-loading-title">{title}</div>
      <div className="mut seer-loading-copy">{message}</div>
      <div className="row gap-8" style={{ justifyContent: "center", marginTop: 18 }}>
        <button className="btn btn-primary" onClick={onRetry}>Try again</button>
        <button className="btn btn-ghost" onClick={onLogout}>Log out</button>
      </div>
    </div>
  );
}

function App() {
  const { user, login, logout, authenticated, ready: privyReady } = usePrivy();
  const { wallets } = useWallets();
  const seer = window.useSeerStore?.() || { auth: null, wallet: null, ready: false, loading: false };

  const [connected, setConnected] = useState(!!seer.auth);
  const [route, setRouteState] = useState(() => routeFromPath(window.location.pathname));
  const [toast, setToast] = useState(null); // { msg, kind: 'info'|'success'|'error' }
  const [navCollapsed, setNavCollapsed] = useState(() => { try { return localStorage.getItem("seerNav") !== "0"; } catch (e) { return true; } });
  const [connectError, setConnectError] = useState(null);
  const connectingRef = useRef(false); // lock - prevents concurrent sign prompts

  const toggleNav = useCallback(() => setNavCollapsed((c) => { const n = !c; try { localStorage.setItem("seerNav", n ? "1" : "0"); } catch (e) {} return n; }), []);
  const setRoute = useCallback((next) => {
    setRouteState(next);
    pushRoute(next);
  }, []);

  // Handle Privy authentication - runs once when the user first authenticates.
  // route/setRoute are intentionally excluded from deps to avoid re-running
  // after navigation, which would trigger duplicate sign prompts.
  useEffect(() => {
    if (!privyReady || !authenticated || !user || wallets.length === 0 || connected) return;
    if (connectingRef.current) return;
    connectingRef.current = true;
    setConnectError(null);
    const wallet = wallets[0];
    const setupConnection = async () => {
      try {
        window.privyWallet = wallet;
        if (wallet.getEthersProvider) {
          window.privyEthersProvider = await wallet.getEthersProvider();
        }
        // Switch embedded wallets (Google/email) to Mantle Sepolia immediately
        // so all subsequent calls are already on the right chain.
        if (wallet.switchChain) {
          try { await wallet.switchChain(5003); } catch (_) {}
          if (wallet.getEthersProvider) {
            window.privyEthersProvider = await wallet.getEthersProvider();
          }
        }
        if (window.SeerAPI) {
          await window.SeerAPI.connectWalletDirect(wallet.address);
          setConnected(true);
          setRoute("agent");
        }
      } catch (error) {
        console.error('Error connecting wallet:', error);
        setConnectError(error.message || "Could not confirm your wallet with the Seer backend.");
        showToast('Failed to connect: ' + error.message, 'error');
      } finally {
        connectingRef.current = false;
      }
    };
    setupConnection();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [privyReady, authenticated, user, wallets, connected]);

  // Always keep window.privyEthersProvider fresh whenever wallets change,
  // independent of the connection flow so the provider is ready for on-chain calls.
  useEffect(() => {
    if (!privyReady || wallets.length === 0) return;
    const wallet = wallets[0];
    window.privyWallet = wallet;
    if (wallet.getEthersProvider) {
      wallet.getEthersProvider().then((p) => {
        window.privyEthersProvider = p;
      }).catch(() => {});
    }
  }, [privyReady, wallets]);

  // Watchdog: if Privy reports authenticated but never delivers a wallet,
  // surface an actionable error instead of leaving the user stuck.
  useEffect(() => {
    if (!privyReady || !authenticated || connected || wallets.length > 0 || connectError) return;
    const timer = setTimeout(() => {
      setConnectError("Your wallet did not finish loading. Log out and back in if this keeps happening.");
    }, 15000);
    return () => clearTimeout(timer);
  }, [privyReady, authenticated, connected, wallets.length, connectError]);

  useEffect(() => {
    const current = routeFromPath(window.location.pathname);
    setRouteState(current);
    if (["/Seer.html", "/index.html"].includes(window.location.pathname)) pushRoute(current, true);
    const onPop = () => setRouteState(routeFromPath(window.location.pathname));
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, []);

  useEffect(() => {
    if (window.SeerAPI) {
      window.SeerAPI.loadPublic().catch((err) => window.SEER?.update({ error: err.message }));
    }
  }, []);

  useEffect(() => {
    if (!connected || !seer.wallet) return;
    if (window.SeerAPI) {
      window.SeerAPI.bootstrap(seer.wallet)
        .then(() => window.SeerLive?.start?.())
        .catch((err) => {
          if (err.status === 401) {
            // Session expired - force re-auth; clearSession already called in request()
            setConnected(false);
          } else {
            showToast(err.message, 'error');
          }
        });
      return () => window.SeerLive?.stop?.();
    }
  }, [connected, seer.wallet]);

  const showToast = useCallback((msg, kind = 'info', txHash = null) => {
    setToast({ msg, kind, txHash });
    // Tx toasts stay longer so the user can click through to explorer
    setTimeout(() => setToast(null), txHash ? 12000 : 5000);
  }, []);

  const onMirror = useCallback((s) => {
    setRoute("agent");
    window.SEER?.update({ pendingIntentText: `Mirror this signal: ${s.head}. ${s.body}` });
    showToast("Intent pre-filled from signal - review and deploy.", 'success');
  }, [setRoute, showToast]);

  const connect = useCallback(async () => {
    if (!authenticated) { login(); return; }
    if (connected) {
      // If a previous bootstrap failed, kick it off again instead of
      // landing the user on a stuck loading screen.
      if (!seer.ready && !seer.loading && seer.wallet && window.SeerAPI) {
        window.SEER?.update({ error: null });
        window.SeerAPI.bootstrap(seer.wallet)
          .then(() => window.SeerLive?.start?.())
          .catch((err) => {
            if (err.status === 401) setConnected(false);
          });
      }
      setRoute("agent");
      return;
    }
    if (connectingRef.current || wallets.length === 0) {
      // Connection still in flight (or Privy hasn't delivered the wallet yet).
      // Navigate to the progress screen - the auto-connect effect finishes the
      // job as soon as the wallet is available.
      setRoute("agent");
      if (wallets.length === 0) showToast("Your wallet is still initializing - one moment.", 'info');
      return;
    }
    connectingRef.current = true;
    setConnectError(null);
    const wallet = wallets[0];
    try {
      window.privyWallet = wallet;
      if (wallet.getEthersProvider) { window.privyEthersProvider = await wallet.getEthersProvider(); }
      if (window.SeerAPI) {
        await window.SeerAPI.connectWalletDirect(wallet.address);
        setConnected(true);
        setRoute("agent");
      }
    } catch (err) {
      setConnectError(err.message || "Could not confirm your wallet with the Seer backend.");
      showToast('Failed to connect: ' + err.message, 'error');
    } finally {
      connectingRef.current = false;
    }
  }, [authenticated, connected, login, wallets, setRoute, showToast, seer.ready, seer.loading, seer.wallet]);

  const disconnect = useCallback(() => {
    window.SeerLive?.stop?.();
    window.SeerAPI?.disconnect?.();
    logout();
    setConnected(false);
  }, [logout]);

  if (!privyReady && route !== "home") return <LoadingScreen message="Starting up." />;
  if (!authenticated) {
    if (route !== "home") pushRoute("home", true);
    return <Landing onEnter={connect} isAuthenticated={false} />;
  }
  if (route === "home") return <Landing onEnter={connect} isAuthenticated={true} />;

  // Privy session exists but the Seer session isn't established yet.
  if (!connected) {
    if (connectError) {
      return (
        <div>
          <ConnectIssueScreen
            title="Couldn't finish signing you in."
            message={connectError}
            onRetry={() => { setConnectError(null); connect(); }}
            onLogout={disconnect}
          />
          <Toast toast={toast} />
        </div>
      );
    }
    return (
      <div>
        <LoadingScreen message="Confirming your wallet with Seer. This can take a few seconds." />
        <Toast toast={toast} />
      </div>
    );
  }

  if (!seer.ready) {
    if (seer.error && !seer.loading) {
      return (
        <div>
          <ConnectIssueScreen
            title="Couldn't load your Seer data."
            message={seer.error + " The backend may be waking up - retrying usually fixes this."}
            onRetry={connect}
            onLogout={disconnect}
          />
          <Toast toast={toast} />
        </div>
      );
    }
    return (
      <div>
        <LoadingScreen message={seer.loading ? "Loading wallet, agents, arena, identity, and settings." : "Preparing your authenticated Seer session."} />
        <Toast toast={toast} />
      </div>
    );
  }

  const rail = route === "feed";

  const screens = {
    feed: <SignalFeed onMirror={onMirror} />,
    agent: <AgentScreen showToast={showToast} />,
    identity: <IdentityScreen showToast={showToast} />,
    arena: <ArenaScreen showToast={showToast} />,
    settings: <SettingsScreen />,
  };

  return (
    <div className={"seer-app" + (rail ? "" : " no-rail") + (navCollapsed ? " nav-collapsed" : "")}>
      <Sidebar route={route} setRoute={setRoute} onDisconnect={disconnect} badge={0} collapsed={navCollapsed} onToggle={toggleNav} />
      <main className={"seer-main" + (route === "agent" ? " agent-mode" : "")}>{screens[route]}</main>
      {rail && <RightRail setRoute={setRoute} riskScore={Math.round(seer.riskScore || 0)} showToast={showToast} />}
      <Toast toast={toast} />
    </div>
  );
}

export default App;
