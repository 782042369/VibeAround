import { useCallback, useEffect, useState } from "react";
import { Splash } from "./Splash";
import Onboarding from "./Onboarding";
import { SetupGuide } from "./SetupGuide";

// ---------------------------------------------------------------------------
// Routing + desktop app shell
// ---------------------------------------------------------------------------

function App() {
  const [route, setRoute] = useState(() => window.location.pathname);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    const onPop = () => setRoute(window.location.pathname);
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, []);

  useEffect(() => setReady(true), []);

  const returnHome = useCallback(() => {
    window.history.replaceState(null, "", "/");
    setRoute("/");
  }, []);

  if (!ready) {
    return <Splash visible />;
  }

  if (route === "/onboarding") {
    return <Onboarding onComplete={returnHome} onExit={returnHome} />;
  }

  return <SetupGuide />;
}

export default App;
