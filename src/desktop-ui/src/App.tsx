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
  const [environmentConfigured, setEnvironmentConfigured] = useState(
    () =>
      typeof window !== "undefined" &&
      window.localStorage.getItem("vibewbz.environmentConfigured") === "1",
  );

  useEffect(() => {
    const onPop = () => setRoute(window.location.pathname);
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, []);

  useEffect(() => setReady(true), []);

  const openOnboarding = useCallback(() => {
    window.history.pushState(null, "", "/onboarding");
    setRoute("/onboarding");
  }, []);

  const completeEnvironmentSetup = useCallback(() => {
    setEnvironmentConfigured(true);
    window.history.pushState(null, "", "/");
    setRoute("/");
  }, []);

  if (!ready) {
    return <Splash visible />;
  }

  if (route === "/onboarding" || !environmentConfigured) {
    return <Onboarding onComplete={completeEnvironmentSetup} />;
  }

  return <SetupGuide onConfigureEnvironment={openOnboarding} />;
}

export default App;
