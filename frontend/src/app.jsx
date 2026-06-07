/* ============================================================
   SEER — app root + routing with Privy auth
   ============================================================ */
import React, { useState, useEffect, useCallback } from 'react';
import { usePrivy, useWallets } from '@privy-io/react-auth';
import { createSmartAccount } from './utils/smartAccount';
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

function LoadingScreen({ message = "Seer is loading authenticated Mantle data from the backend." }) {
  return (
    <div className="seer-loading-screen">
      <div className="serif seer-loading-title">Reading your wallet.</div>
      <div className="mut seer-loading-copy">{message}</div>
      <div className="seer-loading-bar"><i /></div>
    </div>
  );
}

function App() {
  const { user, login, logout, authenticated } = usePrivy();
  const { wallets } = useWallets();
  const seer = window.useSeerStore?.() || { auth: null, wallet: null, ready: false, loading: false };

  const [connected, setConnected] = useState(!!seer.auth);
  const [route, setRouteState] = useState(() => routeFromPath(window.location.pathname));
  const [toast, setToast] = useState(null);
  const [navCollapsed, setNavCollapsed] = useState(() => { try { return localStorage.getItem("seerNav") !== "0"; } catch (e) { return true; } });
  const [smartAccount, setSmartAccount] = useState(null);

  const toggleNav = useCallback(() => setNavCollapsed((c) => { const n = !c; try { localStorage.setItem("seerNav", n ? "1" : "0"); } catch (e) {} return n; }), []);
  const setRoute = useCallback((next) => {
    setRouteState(next);
    pushRoute(next);
  }, []);

  // Handle Privy authentication
  useEffect(() => {
    if (authenticated && user && wallets.length > 0) {
      const wallet = wallets[0];
      const setupSmartAccount = async () => {
        try {
          const smartAcc = await createSmartAccount(wallet);
          setSmartAccount(smartAcc);
          if (window.SeerAPI) {
            const session = await window.SeerAPI.connectWalletDirect(wallet.address);
            setConnected(true);
            if (route === "home") setRoute("agent");
          }
        } catch (error) {
          console.error('Error setting up smart account:', error);
          showToast('Error setting up smart account: ' + error.message);
        }
      };
      setupSmartAccount();
    }
  }, [authenticated, user, wallets, route, setRoute]);

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
        .catch((err) => showToast(err.message));
      return () => window.SeerLive?.stop?.();
    }
  }, [connected, seer.wallet]);

  const showToast = useCallback((msg) => { setToast(msg); setTimeout(() => setToast(null), 2600); }, []);

  const onMirror = useCallback((s) => {
    setRoute("agent");
    window.SEER?.update({ pendingIntentText: `Mirror this signal: ${s.head}. ${s.body}` });
    showToast("Intent pre-filled from signal — review and deploy.");
  }, [setRoute, showToast]);

  const connect = useCallback(async () => {
    if (!authenticated) {
      login();
    }
  }, [authenticated, login]);

  const disconnect = useCallback(() => {
    window.SeerLive?.stop?.();
    window.SeerAPI?.disconnect?.();
    logout();
    setConnected(false);
  }, [logout]);

  if (route === "home" || !authenticated) return <Landing onEnter={connect} />;

  if (!seer.ready) {
    return (
      <div>
        <LoadingScreen message={seer.loading ? "Loading wallet, agents, arena, identity, and settings." : "Preparing your authenticated Seer session."} />
        {toast && <div className="seer-toast">{toast}</div>}
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
      {rail && <RightRail setRoute={setRoute} riskScore={Math.round(seer.riskScore || 0)} />}
      {toast && <div className="seer-toast">{toast}</div>}
    </div>
  );
}

export default App;
