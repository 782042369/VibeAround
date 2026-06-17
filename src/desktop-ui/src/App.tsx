import { useCallback, useEffect, useRef, useState } from "react";
import { Rocket } from "lucide-react";
import { useI18n } from "@va/i18n";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { Splash } from "./Splash";
import Onboarding from "./Onboarding";
import { Launch } from "./Launch";
import {
  getLauncherPreferences,
  rescanAgentEntries,
  rescanDesktopAppEntries,
  type LauncherPreferences,
} from "./Launch/api";
import { LanguageMenu } from "./components/LanguageMenu";
import { cn } from "./lib/utils";
import { UpdateIndicator } from "./UpdateIndicator";

// ---------------------------------------------------------------------------
// Routing + desktop app shell
// ---------------------------------------------------------------------------

function App() {
  const [route, setRoute] = useState(() => window.location.pathname);
  const [startupScanDone, setStartupScanDone] = useState(false);

  useEffect(() => {
    const onPop = () => setRoute(window.location.pathname);
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, []);

  useEffect(() => {
    let cancelled = false;
    void Promise.allSettled([rescanAgentEntries(), rescanDesktopAppEntries()]).then(
      () => {
        if (!cancelled) setStartupScanDone(true);
      },
    );
    return () => {
      cancelled = true;
    };
  }, []);

  if (!startupScanDone) {
    return <Splash visible />;
  }

  if (route === "/onboarding") {
    return <Onboarding />;
  }

  return <DesktopApp />;
}

type DesktopAppPage = "launch";

function DesktopApp() {
  const { t } = useI18n();
  const isMacTitlebar =
    typeof navigator !== "undefined" && /Mac/.test(navigator.platform);
  const [page, setPage] = useState<DesktopAppPage>("launch");
  const [launcherPrefs, setLauncherPrefs] =
    useState<LauncherPreferences | null>(null);
  const [launcherPrefsLoaded, setLauncherPrefsLoaded] = useState(false);
  const everHadData = useRef(false);

  const refreshLauncherPrefs = useCallback(() => {
    void getLauncherPreferences()
      .then((prefs) => {
        setLauncherPrefs(prefs);
        setLauncherPrefsLoaded(true);
      })
      .catch(() => {
        setLauncherPrefs(null);
        setLauncherPrefsLoaded(true);
      });
  }, []);

  const launchEnabled = !launcherPrefsLoaded
    ? false
    : launcherPrefs
      ? launcherPrefs.enabledAgents.length > 0
      : true;
  const launchDisabledReason = !launcherPrefsLoaded
    ? t("Loading launch settings")
    : !launchEnabled
      ? t("No launch agents enabled")
      : null;
  const effectivePage = page;

  if (launcherPrefsLoaded) everHadData.current = true;

  useEffect(() => {
    refreshLauncherPrefs();
  }, [refreshLauncherPrefs]);

  const showSplash = !everHadData.current && !launcherPrefsLoaded;
  if (showSplash) return <Splash visible />;

  return (
    <div className="h-full flex flex-col">
      <header
        className={cn(
          "relative flex h-12 items-center justify-between pr-3 border-b border-border shrink-0",
          isMacTitlebar ? "pl-[82px]" : "pl-3",
        )}
      >
        <div
          data-tauri-drag-region
          aria-hidden="true"
          className="absolute inset-0 z-0"
        />
        <div className="relative z-10 flex min-w-0 items-center gap-1.5 whitespace-nowrap">
          <VibeWbzMark />
          <span className="text-[13px] font-semibold text-foreground">
            VibeWbz
          </span>
          <span className="font-mono text-[10px] text-muted-foreground/60">
            @{__APP_VERSION_LABEL__}
          </span>
          <UpdateIndicator />
        </div>
        <div className="absolute left-1/2 top-1/2 z-10 -translate-x-1/2 -translate-y-1/2">
          <Tabs
            value={effectivePage}
            onValueChange={(value) => {
              if (value === "launch" && !launchEnabled) return;
              setPage(value as DesktopAppPage);
            }}
            className="contents"
          >
            <TabsList className="!h-8 rounded-md p-1">
              <TooltipProvider>
                {launchDisabledReason ? (
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <span
                        className="inline-flex cursor-not-allowed"
                        tabIndex={0}
                        role="button"
                        aria-disabled="true"
                        aria-label={launchDisabledReason}
                        title={launchDisabledReason}
                      >
                        <TabsTrigger
                          value="launch"
                          disabled
                          className="!h-6 gap-1 px-2 text-xs [&_svg:not([class*='size-'])]:!size-3.5"
                        >
                          <Rocket /> {t("Launch")}
                        </TabsTrigger>
                      </span>
                    </TooltipTrigger>
                    <TooltipContent side="bottom">
                      {launchDisabledReason}
                    </TooltipContent>
                  </Tooltip>
                ) : (
                  <TabsTrigger
                    value="launch"
                    className="!h-6 gap-1 px-2 text-xs [&_svg:not([class*='size-'])]:!size-3.5"
                  >
                    <Rocket /> {t("Launch")}
                  </TabsTrigger>
                )}
              </TooltipProvider>
            </TabsList>
          </Tabs>
        </div>
        <div className="relative z-10 flex items-center gap-2">
          <LanguageMenu />
        </div>
      </header>

      <div className="flex min-h-0 flex-1 flex-col">
        <div className="min-h-0 flex-1">
          <Launch />
        </div>
        <footer className="shrink-0 border-t border-border px-3 py-1.5 text-center text-[11px] leading-4 text-muted-foreground">
          {t("仅供社区学习交流使用，所有安装源均来自官方来源。")}
        </footer>
      </div>
    </div>
  );
}

export default App;

function VibeWbzMark() {
  return (
    <img
      src="/brand/vibewbz-mark.svg"
      alt=""
      className="h-5 w-5 shrink-0"
      aria-hidden="true"
      draggable={false}
    />
  );
}
