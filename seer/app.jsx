/* ============================================================
   SEER — app root + routing
   ============================================================ */

function Placeholder({ title, sub }) {
  return (
    <div className="seer-screen">
      <header className="seer-screen-head"><div><h1 className="serif seer-h1">{title}</h1><p className="seer-screen-sub">{sub}</p></div></header>
      <EmptyState icon="spark" title="Coming together next." body="This screen is part of the build in progress — the system, motion, and copy you see here carry straight into it." />
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
      <SeerOrb size={54} thinking />
      <div className="serif seer-loading-title">Reading your wallet.</div>
      <div className="mut seer-loading-copy">{message}</div>
      <div className="seer-loading-bar"><i /></div>
    </div>
  );
}

function App() {
  const seer = window.useSeerStore();
  const [connected, setConnected] = useState(!!seer.auth);
  const [route, setRouteState] = useState(() => routeFromPath(window.location.pathname));
  const [toast, setToast] = useState(null);
  const [navCollapsed, setNavCollapsed] = useState(() => { try { return localStorage.getItem("seerNav") !== "0"; } catch (e) { return true; } });
  const toggleNav = useCallback(() => setNavCollapsed((c) => { const n = !c; try { localStorage.setItem("seerNav", n ? "1" : "0"); } catch (e) {} return n; }), []);

  const setRoute = useCallback((next) => {
    setRouteState(next);
    pushRoute(next);
  }, []);

  useEffect(() => {
    const current = routeFromPath(window.location.pathname);
    setRouteState(current);
    if (["/Seer.html", "/index.html"].includes(window.location.pathname)) pushRoute(current, true);
    const onPop = () => setRouteState(routeFromPath(window.location.pathname));
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, []);

  useEffect(() => {
    window.SeerAPI.loadPublic().catch((err) => window.SEER.update({ error: err.message }));
  }, []);

  useEffect(() => {
    if (!connected || !seer.wallet) return;
    window.SeerAPI.bootstrap(seer.wallet)
      .then(() => window.SeerLive.start())
      .catch((err) => showToast(err.message));
    return () => window.SeerLive.stop();
  }, [connected, seer.wallet]);

  const showToast = useCallback((msg) => { setToast(msg); setTimeout(() => setToast(null), 2600); }, []);

  const onMirror = useCallback((s) => {
    setRoute("agent");
    window.SEER.update({ pendingIntentText: `Mirror this signal: ${s.head}. ${s.body}` });
    showToast("Intent pre-filled from signal — review and deploy.");
  }, [setRoute, showToast]);

  const connect = useCallback(async () => {
    const session = await window.SeerAPI.connectWallet();
    setConnected(true);
    if (route === "home") setRoute("agent");
    return session;
  }, [route, setRoute]);

  const disconnect = useCallback(() => {
    window.SeerLive.stop();
    window.SeerAPI.disconnect();
    setConnected(false);
  }, []);

  if (route === "home") return <Landing onEnter={connect} />;

  if (!connected) return <Landing onEnter={connect} />;

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
    agent: window.AgentScreen ? <AgentScreen showToast={showToast} /> : <Placeholder title="My Agent" sub="Tell Seer what to do in plain English." />,
    identity: window.IdentityScreen ? <IdentityScreen showToast={showToast} /> : <Placeholder title="My Identity" sub="See how you compare to smart money." />,
    arena: window.ArenaScreen ? <ArenaScreen showToast={showToast} /> : <Placeholder title="The Arena" sub="Bet against Seer's predictions." />,
    settings: window.SettingsScreen ? <SettingsScreen /> : <Placeholder title="Settings" sub="Notifications, agent permissions, identity." />,
  };

  return (
    <div className={"seer-app" + (rail ? "" : " no-rail") + (navCollapsed ? " nav-collapsed" : "")}>
      <Sidebar route={route} setRoute={setRoute} onDisconnect={disconnect} badge={0} collapsed={navCollapsed} onToggle={toggleNav} />
      <main className={"seer-main" + (route === "agent" ? " agent-mode" : "")}>{screens[route]}</main>
      {rail && <RightRail setRoute={setRoute} riskScore={Math.round(seer.riskScore || 0)} />}
      {toast && <div className="seer-toast">{toast}</div>}
      {window.SeerTweaks && <SeerTweaks />}
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")).render(<App />);
