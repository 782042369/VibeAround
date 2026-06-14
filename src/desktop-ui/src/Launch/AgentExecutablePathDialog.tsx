import { useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Check, RefreshCw } from "lucide-react";
import { useI18n } from "@va/i18n";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import type {
  AgentExecutableCandidate,
  AgentExecutableResolution,
  AgentSummary,
} from "./api";
import type { AgentLaunchPreference } from "./types";

interface Props {
  agent: AgentSummary | null;
  preference?: AgentLaunchPreference;
  executableResolution?: AgentExecutableResolution | null;
  executableLoading?: boolean;
  busy: boolean;
  onClose: () => void;
  onSaveExecutablePath: (path: string | null) => Promise<void>;
  onRefreshExecutableResolution?: () => Promise<void>;
  onUpdateAgent?: () => Promise<void>;
}

type ClientOs = "macos" | "windows" | "linux";

function detectClientOs(): ClientOs {
  const platform = (
    typeof navigator === "undefined" ? "" : navigator.platform || ""
  ).toLowerCase();
  const ua = (
    typeof navigator === "undefined" ? "" : navigator.userAgent || ""
  ).toLowerCase();
  const source = `${platform} ${ua}`;
  if (source.includes("win")) return "windows";
  if (source.includes("mac")) return "macos";
  return "linux";
}

function configuredPath(
  preference?: AgentLaunchPreference,
  resolution?: AgentExecutableResolution | null,
): string {
  return (
    resolution?.configuredPath ??
    resolution?.selected?.path ??
    preference?.executable?.path ??
    preference?.executablePath ??
    ""
  );
}

function pathMatchesCandidate(
  path: string,
  candidate: AgentExecutableCandidate,
): boolean {
  const trimmed = path.trim();
  return (
    trimmed.length > 0 &&
    (trimmed === candidate.path || trimmed === (candidate.realpath ?? ""))
  );
}

export function AgentExecutablePathDialog({
  agent,
  preference,
  executableResolution,
  executableLoading = false,
  busy,
  onClose,
  onSaveExecutablePath,
  onRefreshExecutableResolution,
  onUpdateAgent,
}: Props) {
  const { t } = useI18n();
  const initialPath = useMemo(
    () => configuredPath(preference, executableResolution),
    [preference, executableResolution],
  );
  const [executablePath, setExecutablePath] = useState(initialPath);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [updatingAgent, setUpdatingAgent] = useState(false);

  useEffect(() => {
    setExecutablePath(initialPath);
    setSaveError(null);
    setUpdatingAgent(false);
  }, [agent?.id, initialPath]);

  if (!agent) return null;

  const clientOs = detectClientOs();
  const isDesktopApp = agent.direct_only;
  const executableDirty = executablePath.trim() !== initialPath;
  const draftCandidate =
    executableResolution?.candidates.find((candidate) =>
      pathMatchesCandidate(executablePath, candidate),
    ) ?? null;
  const selectedCandidate = executableResolution?.selected ?? null;
  const updateCandidate = executableDirty
    ? null
    : (draftCandidate ?? selectedCandidate);
  const updateCommand = updateCandidate?.updateCommand ?? null;

  async function save() {
    setSaveError(null);
    try {
      await onSaveExecutablePath(executablePath.trim() || null);
      onClose();
    } catch (error) {
      setSaveError(error instanceof Error ? error.message : String(error));
    }
  }

  async function chooseExecutable() {
    const selected = await open({
      directory: false,
      multiple: false,
      title: isDesktopApp
        ? t("Choose desktop app executable")
        : t("Choose agent executable"),
      filters:
        clientOs === "windows"
          ? [{ name: "Executable", extensions: ["exe"] }]
          : undefined,
    });
    const path = Array.isArray(selected) ? selected[0] : selected;
    if (path) setExecutablePath(path);
  }

  async function updateAgent() {
    if (!onUpdateAgent) return;
    setSaveError(null);
    setUpdatingAgent(true);
    try {
      await onUpdateAgent();
      await onRefreshExecutableResolution?.();
    } catch (error) {
      setSaveError(error instanceof Error ? error.message : String(error));
    } finally {
      setUpdatingAgent(false);
    }
  }

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="!flex max-h-[calc(100vh-64px)] w-[min(660px,calc(100vw-28px))] max-w-[calc(100vw-28px)] flex-col overflow-hidden p-0 sm:max-w-[min(660px,calc(100vw-28px))]">
        <DialogHeader className="shrink-0 border-b border-border px-5 py-3 pr-12">
          <DialogTitle className="text-lg">
            {isDesktopApp
              ? t("{{agent}} app path", { agent: agent.display_name })
              : t("{{agent}} CLI launch path", { agent: agent.display_name })}
          </DialogTitle>
          <DialogDescription className="sr-only">
            {isDesktopApp
              ? t("Choose the desktop app executable.")
              : t("Choose the CLI path used by Launch and ACP.")}
          </DialogDescription>
        </DialogHeader>

        <div className="min-h-0 flex-1 space-y-3 overflow-y-auto px-5 py-3">
          <section className="space-y-2">
            <div>
              <div className="text-[10px] font-semibold uppercase tracking-[0.16em] text-muted-foreground/70">
                {isDesktopApp ? t("Desktop app") : t("Executable")}
              </div>
              <div className="mt-0.5 text-xs text-muted-foreground">
                {isDesktopApp
                  ? t("Use a specific executable when auto-detect cannot find the app.")
                  : t("Choose the CLI path used by Launch and ACP.")}
              </div>
            </div>

            {!isDesktopApp && (
              <div className="space-y-1.5 rounded-md border border-border bg-background/60 p-2">
                {executableLoading ? (
                  <div className="text-[11px] text-muted-foreground">
                    {t("Checking path")}
                  </div>
                ) : executableResolution?.candidates.length ? (
                  executableResolution.candidates.map((candidate) => {
                    const selected = pathMatchesCandidate(
                      executablePath,
                      candidate,
                    );
                    return (
                      <button
                        key={`${candidate.source}:${candidate.path}`}
                        type="button"
                        disabled={busy}
                        className={`flex w-full min-w-0 items-center gap-2 rounded-md border px-2 py-1.5 text-left transition-colors ${
                          selected
                            ? "border-primary/45 bg-primary/10"
                            : "border-border bg-card hover:border-primary/30"
                        }`}
                        onClick={() => setExecutablePath(candidate.path)}
                      >
                        <span className="flex h-5 w-5 shrink-0 items-center justify-center text-primary">
                          {selected ? (
                            <Check className="h-3.5 w-3.5" />
                          ) : (
                            <span className="h-2 w-2 rounded-full border border-muted-foreground/50" />
                          )}
                        </span>
                        <span className="min-w-0 flex-1">
                          <span className="block truncate font-mono text-[11px] [font-variant-ligatures:none]">
                            {candidate.path}
                          </span>
                          <span className="block truncate text-[10px] text-muted-foreground">
                            {candidate.sourceLabel}
                            {candidate.version ? ` · ${candidate.version}` : ""}
                          </span>
                        </span>
                      </button>
                    );
                  })
                ) : (
                  <div className="text-[11px] text-muted-foreground">
                    {t("No executable candidates found")}
                  </div>
                )}
              </div>
            )}

            <div className="flex gap-1.5">
              <Input
                value={executablePath}
                disabled={busy}
                placeholder={
                  clientOs === "windows"
                    ? "C:\\Path\\To\\Agent.exe"
                    : isDesktopApp
                      ? "/Applications/App.app/Contents/MacOS/App"
                      : "/opt/homebrew/bin/agent"
                }
                className="!h-8 min-h-8 max-h-8 font-mono !text-[11px] leading-4 placeholder:!text-[11px] md:!text-[11px] [font-variant-ligatures:none]"
                onChange={(event) => setExecutablePath(event.target.value)}
              />
              <Button
                type="button"
                variant="outline"
                size="sm"
                disabled={busy}
                className="h-8 px-2.5 text-xs"
                onClick={() => void chooseExecutable()}
              >
                {t("Choose")}
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                disabled={busy || !executablePath}
                className="h-8 px-2.5 text-xs"
                onClick={() => setExecutablePath("")}
              >
                {t("Clear")}
              </Button>
            </div>

            {!isDesktopApp && (
              <div className="flex flex-wrap gap-1.5">
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  disabled={busy || executableLoading}
                  className="h-7 px-2 text-xs"
                  onClick={() => void onRefreshExecutableResolution?.()}
                >
                  <RefreshCw className="h-3.5 w-3.5" />
                  {t("Scan")}
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  disabled={
                    busy ||
                    updatingAgent ||
                    executableDirty ||
                    !updateCommand
                  }
                  className="h-7 px-2 text-xs"
                  title={
                    executableDirty
                      ? t("Save this path before updating")
                      : (updateCommand ?? t("No update command"))
                  }
                  onClick={() => void updateAgent()}
                >
                  <RefreshCw className="h-3.5 w-3.5" />
                  {updatingAgent ? t("Updating") : t("Update")}
                </Button>
              </div>
            )}
          </section>

          {saveError && (
            <div className="text-xs text-destructive">{saveError}</div>
          )}
        </div>

        <DialogFooter className="shrink-0 border-t border-border px-5 py-3">
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={busy}
            onClick={onClose}
          >
            {t("Cancel")}
          </Button>
          <Button
            type="button"
            size="sm"
            disabled={busy || !executableDirty}
            onClick={() => void save()}
          >
            {t("Save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
