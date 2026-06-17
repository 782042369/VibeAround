import { useCallback, useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  FolderOpen,
  Monitor,
  Pencil,
  Rocket,
  Terminal,
} from "lucide-react";
import { useI18n } from "@va/i18n";

import { TooltipProvider } from "@/components/ui/tooltip";
import { AgentRailButton, TooltipButton } from "./LaunchBuilderPrimitives";
import {
  ProfilePanel,
  WorkspacePanel,
} from "./LaunchBuilderPanels";
import {
  AgentSummaryHeader,
  LaunchSummaryPill,
  ProfileInfoPanel,
  SelectorPopup,
  type SelectorPopupId,
} from "./LaunchSummary";
import {
  createProfile,
  deleteProfile,
  getLauncherPreferences,
  getProfile,
  launchProfile,
  listAgents,
  listLauncherWorkspaces,
  listProfiles,
  removeLauncherWorkspace,
  reorderLauncherWorkspaces,
  reorderProfiles,
  getDesktopAppEntries,
  getAgentExecutableLatest,
  getAgentExecutableResolution,
  updateLauncherAgent,
  setProfileConnection,
  setLauncherAgentExecutablePath,
  setLauncherAgentProfile,
  setLauncherDefault,
  setLauncherSelectedAgent,
  setLauncherWorkspace,
  type AgentSummary,
  type AgentExecutableLatest,
  type AgentExecutableResolution,
  type DesktopAppDetectionFile,
  type LauncherPreferences,
  type WorkspaceOption,
} from "./api";
import { buildProfileCopyDraft } from "./profileClone";
import {
  connectionAgentId,
  agentProfileId,
  currentWorkspace,
  agentWorkspace,
  isSelectionLaunchable,
  isSortableWorkspace,
  mergeOrderedSubset,
  moveItemBefore,
  profileById,
  profileSupportsAgent,
  profileSummary,
  selectionUnavailableReason,
  type ProfileChoice,
} from "./launchModel";
import type {
  ConnectionAgentId,
  ProfileConnectionPreference,
  ProfileSummary,
} from "./types";
import { AgentExecutablePathDialog } from "./AgentExecutablePathDialog";

const AGENT_ORDER = [
  "codex",
  "codex-desktop",
  "claude",
  "claude-desktop",
];

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

interface Props {
  profiles: ProfileSummary[];
  prefs: LauncherPreferences | null;
  onPrefsChange: (prefs: LauncherPreferences) => void;
  onProfilesChange: (profiles: ProfileSummary[]) => void;
  onNewProfile: () => void;
  onEditProfile: (profile: ProfileSummary) => void;
  onConnectionSettings: (
    profile: ProfileSummary,
    agentId: ConnectionAgentId,
  ) => void;
  onError: (message: string | null) => void;
  onToast: (message: string | null) => void;
}

export function AgentLaunchBuilder({
  profiles,
  prefs,
  onPrefsChange,
  onProfilesChange,
  onNewProfile,
  onEditProfile,
  onConnectionSettings,
  onError,
  onToast,
}: Props) {
  const { locale, t } = useI18n();
  const [agents, setAgents] = useState<AgentSummary[]>([]);
  const [agentId, setAgentId] = useState<string>("");
  const [profileChoiceAgentId, setProfileChoiceAgentId] = useState<string>("");
  const [profileChoice, setProfileChoice] = useState<ProfileChoice | null>(null);
  const [openSelector, setOpenSelector] = useState<SelectorPopupId | null>(
    null,
  );
  const [workspaceOptions, setWorkspaceOptions] = useState<
    WorkspaceOption[] | null
  >(null);
  const [workspacesLoading, setWorkspacesLoading] = useState(false);
  const [pathAgent, setPathAgent] = useState<AgentSummary | null>(null);
  const [agentExecutable, setAgentExecutable] =
    useState<AgentExecutableResolution | null>(null);
  const [agentExecutableLoading, setAgentExecutableLoading] = useState(false);
  const [desktopAppEntries, setDesktopAppEntries] =
    useState<DesktopAppDetectionFile | null>(null);
  const [busy, setBusy] = useState(false);

  const enabledAgentKey = prefs?.enabledAgents.join("|") ?? "";
  const enabledAgents = useMemo(
    () => (prefs ? new Set(prefs.enabledAgents) : null),
    [enabledAgentKey],
  );
  const viewPrefs = useMemo<LauncherPreferences | null>(() => {
    if (!prefs) return null;
    return {
      ...prefs,
      workspace: agentWorkspace(prefs, agentId),
      workspaceOptions: workspaceOptions ?? prefs.workspaceOptions,
    };
  }, [prefs, workspaceOptions, agentId]);

  useEffect(() => {
    void Promise.all([
      listAgents(),
      getDesktopAppEntries().catch(() => null),
    ])
      .then(([items, desktopApps]) => {
        setDesktopAppEntries(desktopApps);
        const rank = new Map(AGENT_ORDER.map((id, index) => [id, index]));
        const visible = items.filter((agent) => {
          if (!rank.has(agent.id)) return false;
          if (agent.direct_only) return true;
          return enabledAgents ? enabledAgents.has(agent.id) : true;
        });
        const ordered = [...visible].sort(
          (a, b) => (rank.get(a.id) ?? 999) - (rank.get(b.id) ?? 999),
        );
        setAgents(ordered);
      })
      .catch((error) =>
        onError(error instanceof Error ? error.message : String(error)),
      );
  }, [enabledAgents, onError]);

  useEffect(() => {
    if (!prefs) return;
    if (agentId && agents.some((agent) => agent.id === agentId)) return;
    const preferredAgent = agents.some(
      (agent) => agent.id === prefs.selectedAgent,
    )
      ? prefs.selectedAgent
      : agents.some((agent) => agent.id === prefs.defaultAgent)
        ? prefs.defaultAgent
      : (agents[0]?.id ?? "");
    setAgentId(preferredAgent);
  }, [agentId, agents, prefs]);

  useEffect(() => {
    if (!prefs || !agentId) return;
    setProfileChoice((current) => {
      if (profileChoiceAgentId === agentId && current) {
        if (
          profileSupportsAgent(
            profileById(profiles, current.profileId),
            agentId,
            prefs,
          )
        ) {
          return current;
        }
      }

      const defaultProfileId = agentProfileId(prefs, agentId);
      if (
        defaultProfileId &&
        profileSupportsAgent(
          profileById(profiles, defaultProfileId),
          agentId,
          prefs,
        )
      ) {
        return { kind: "profile", profileId: defaultProfileId };
      }
      const firstSupportedProfile = profiles.find((profile) =>
        profileSupportsAgent(profile, agentId, prefs),
      );
      return firstSupportedProfile
        ? { kind: "profile", profileId: firstSupportedProfile.id }
        : null;
    });
    setProfileChoiceAgentId(agentId);
  }, [agentId, prefs, profileChoiceAgentId, profiles]);

  const refreshAgentExecutable = useCallback(
    async (targetAgentId = agentId) => {
      if (!targetAgentId) {
        setAgentExecutable(null);
        return;
      }
      if (agents.find((agent) => agent.id === targetAgentId)?.direct_only) {
        setAgentExecutable(null);
        setAgentExecutableLoading(false);
        return;
      }
      setAgentExecutableLoading(true);
      try {
        const resolution = await getAgentExecutableResolution(targetAgentId);
        if (targetAgentId === agentId) {
          setAgentExecutable(resolution);
        }
      } catch (error) {
        if (targetAgentId === agentId) {
          setAgentExecutable(null);
          onError(error instanceof Error ? error.message : String(error));
        }
      } finally {
        if (targetAgentId === agentId) {
          setAgentExecutableLoading(false);
        }
      }
    },
    [agentId, agents, onError],
  );

  useEffect(() => {
    void refreshAgentExecutable(agentId);
  }, [agentId, refreshAgentExecutable]);

  useEffect(() => {
    if (!prefs || !agentId) {
      setWorkspaceOptions(null);
      setWorkspacesLoading(false);
      return;
    }

    let cancelled = false;
    setWorkspacesLoading(true);
    void listLauncherWorkspaces(agentId)
      .then((items) => {
        if (!cancelled) setWorkspaceOptions(items);
      })
      .catch((error) => {
        if (!cancelled) {
          onError(error instanceof Error ? error.message : String(error));
        }
      })
      .finally(() => {
        if (!cancelled) setWorkspacesLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [prefs, agentId, onError]);

  const selectedAgent = agents.find((agent) => agent.id === agentId);
  const selectedAgentIsDirectOnly = Boolean(selectedAgent?.direct_only);
  const selectedProfile =
    profileChoice?.kind === "profile"
      ? profileById(profiles, profileChoice.profileId)
      : null;
  const profileOptions = profiles;
  const selectedWorkspace = currentWorkspace(viewPrefs);
  const selectionLaunchable = viewPrefs
    ? profileChoice
      ? isSelectionLaunchable(profileChoice, selectedProfile, agentId, viewPrefs)
      : false
    : false;
  const selectionDisabledReason = viewPrefs
    ? profileChoice
      ? selectionUnavailableReason(
        profileChoice,
        selectedProfile,
        agentId,
        viewPrefs,
        t,
      )
      : t("Configure gateway first")
    : t("Loading…");
  const launchDisabledReason = busy
    ? t("Launch is already in progress")
    : selectionDisabledReason;

  async function refreshPrefs() {
    onPrefsChange(await getLauncherPreferences());
  }

  async function refreshProfiles() {
    onProfilesChange(await listProfiles());
  }

  async function chooseAgent(nextAgentId: string) {
    setAgentId(nextAgentId);
    onError(null);
    try {
      await setLauncherSelectedAgent(nextAgentId);
      await refreshPrefs();
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    }
  }

  async function chooseProfileChoice(choice: ProfileChoice) {
    setProfileChoice(choice);
    if (!agentId) return;
    onError(null);
    try {
      await setLauncherAgentProfile(
        agentId,
        choice.profileId,
      );
      await refreshPrefs();
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    }
  }

  async function chooseProfileApiType(profile: ProfileSummary, apiType: string) {
    const connectionId = connectionAgentId(agentId);
    if (!viewPrefs || !connectionId) return;
    const current = viewPrefs.profileConnections[profile.id]?.[connectionId] ?? {};
    onError(null);
    try {
      await setProfileConnection(profile.id, connectionId, {
        ...current,
        selectedApiType: apiType,
      });
      await refreshPrefs();
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    }
  }

  async function chooseWorkspace(path: string) {
    if (!prefs || !agentId || path === agentWorkspace(prefs, agentId)) {
      return;
    }
    setBusy(true);
    onError(null);
    try {
      await setLauncherWorkspace(path, agentId);
      await refreshPrefs();
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }

  async function chooseFolder() {
    if (!agentId) return;
    setBusy(true);
    onError(null);
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: t("Choose Launch Workspace"),
      });
      const path = Array.isArray(selected) ? selected[0] : selected;
      if (!path) return;
      await setLauncherWorkspace(path, agentId);
      await refreshPrefs();
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }

  async function makeDefault(choice: ProfileChoice) {
    setBusy(true);
    onError(null);
    try {
      await setLauncherDefault(agentId, choice.profileId);
      await refreshPrefs();
      onToast(t("VibeWbz default updated"));
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }

  async function removeWorkspace(path: string, label: string) {
    if (!window.confirm(t('Remove workspace "{{label}}"?', { label }))) return;
    setBusy(true);
    onError(null);
    try {
      await removeLauncherWorkspace(path);
      await refreshPrefs();
      onToast(t("Workspace removed"));
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }

  async function reorderWorkspace(fromPath: string, toPath: string) {
    if (!viewPrefs || fromPath === toPath) return;
    const reorderablePaths = viewPrefs.workspaceOptions
      .filter(isSortableWorkspace)
      .map((workspace) => workspace.path);
    const nextPaths = moveItemBefore(reorderablePaths, fromPath, toPath);
    if (nextPaths === reorderablePaths) return;
    setBusy(true);
    onError(null);
    try {
      await reorderLauncherWorkspaces(nextPaths);
      await refreshPrefs();
      onToast(t("Workspace order updated"));
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }

  async function removeProfile(profile: ProfileSummary) {
    if (
      !window.confirm(
        t('Delete profile "{{label}}"?', { label: profile.label }),
      )
    )
      return;
    setBusy(true);
    onError(null);
    try {
      await deleteProfile(profile.id);
      await Promise.all([refreshProfiles(), refreshPrefs()]);
      onToast(t("Profile deleted"));
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }

  async function duplicateProfile(profile: ProfileSummary) {
    if (!prefs) return;
    setBusy(true);
    onError(null);
    let copiedProfileId: string | null = null;
    try {
      const fullProfile = await getProfile(profile.id);
      const copiedProfile = await createProfile(
        buildProfileCopyDraft(
          fullProfile,
          locale === "zh-CN" ? "副本" : "Copy",
          profiles.map((candidate) => candidate.label),
        ),
      );
      copiedProfileId = copiedProfile.id;
      const sourceConnections = prefs.profileConnections[profile.id] ?? {};
      const connectionCopies = Object.entries(sourceConnections)
        .filter((entry): entry is [ConnectionAgentId, ProfileConnectionPreference] =>
          Boolean(entry[1]),
        )
        .map(([connectionAgentId, preference]) =>
          setProfileConnection(
            copiedProfile.id,
            connectionAgentId,
            structuredClone(preference),
          ),
        );
      const connectionResults = await Promise.allSettled(connectionCopies);
      const failedConnection = connectionResults.find(
        (result) => result.status === "rejected",
      );
      if (failedConnection) {
        throw failedConnection.reason;
      }
      copiedProfileId = null;
      await Promise.all([refreshProfiles(), refreshPrefs()]);
      onToast(t("Profile duplicated"));
    } catch (error) {
      let message = errorMessage(error);
      if (copiedProfileId) {
        try {
          await deleteProfile(copiedProfileId);
        } catch (rollbackError) {
          message = `${message}. ${t("Rollback failed")}: ${errorMessage(rollbackError)}`;
        }
        await Promise.allSettled([refreshProfiles(), refreshPrefs()]);
      }
      onError(message);
    } finally {
      setBusy(false);
    }
  }

  async function reorderProfile(fromId: string, toId: string) {
    if (fromId === toId) return;
    const visibleIds = profileOptions.map((profile) => profile.id);
    const movedVisibleIds = moveItemBefore(visibleIds, fromId, toId);
    if (movedVisibleIds === visibleIds) return;
    const nextIds = mergeOrderedSubset(
      profiles.map((profile) => profile.id),
      new Set(visibleIds),
      movedVisibleIds,
    );
    setBusy(true);
    onError(null);
    try {
      await reorderProfiles(nextIds);
      await refreshProfiles();
      onToast(t("Profile order updated"));
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }

  async function launchSelected() {
    if (!agentId) return;
    if (profileChoice?.kind !== "profile") {
      onError(t("Configure gateway first"));
      return;
    }
    const launchedAgentId = agentId;
    setBusy(true);
    onError(null);
    try {
      await launchProfile(profileChoice.profileId, launchedAgentId);
      onToast(t("Launch opened"));
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }

  async function saveAgentExecutablePath(path: string | null) {
    if (!pathAgent) return;
    const targetAgent = pathAgent;
    onError(null);
    try {
      await setLauncherAgentExecutablePath(targetAgent.id, path);
      void refreshPrefs().catch((error) => {
        onError(error instanceof Error ? error.message : String(error));
      });
      onToast(t("Agent launch path updated"));
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
      throw error;
    }
  }

  async function updateAgentExecutable(executablePath: string) {
    if (!pathAgent) return;
    const targetAgent = pathAgent;
    setBusy(true);
    onError(null);
    try {
      await updateLauncherAgent(targetAgent.id, executablePath);
      await refreshAgentExecutable(targetAgent.id);
      onToast(t("Agent updated"));
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
      throw error;
    } finally {
      setBusy(false);
    }
  }

  const checkAgentExecutableLatest = useCallback(
    async (executablePath: string): Promise<AgentExecutableLatest> => {
      if (!pathAgent) throw new Error("No agent selected");
      return getAgentExecutableLatest(pathAgent.id, executablePath);
    },
    [pathAgent],
  );

  function openAgentPathDialog(agent: AgentSummary) {
    setPathAgent(agent);
    if (!agent.direct_only) {
      void refreshAgentExecutable(agent.id);
    }
  }

  if (!viewPrefs || agents.length === 0 || !selectedAgent) {
    if (prefs?.enabledAgents.length === 0) {
      return (
        <p className="p-3 text-xs text-muted-foreground">
          {t("No launch agents enabled")}
        </p>
      );
    }
    return <p className="p-3 text-xs text-muted-foreground">{t("Loading…")}</p>;
  }

  const selectedProfileSummary = selectedProfile
    ? profileSummary(selectedProfile, agentId, viewPrefs, t)
    : {
        title: t("Default gateway"),
        detail: selectedAgentIsDirectOnly
          ? t("Open desktop app")
          : t("Use gateway key"),
        bridge: false,
        route: selectedAgentIsDirectOnly
          ? t("Desktop app")
          : t("Gateway"),
      };
  const selectedAgentPreference = viewPrefs.agentPreferences[agentId];
  const showLaunchControls = !selectedAgentIsDirectOnly;
  const desktopAppEntryForAgent = (targetAgentId: string) =>
    desktopAppEntries?.apps[targetAgentId]?.entry;
  const desktopAppPathForAgent = (targetAgentId: string): string | undefined => {
    const app = desktopAppEntries?.apps[targetAgentId];
    return app?.entry?.path ?? app?.launchCommand;
  };
  const desktopAppLaunchTargetLabel = (
    targetAgentId: string,
    target: string,
  ): string => {
    const entry = desktopAppEntryForAgent(targetAgentId);
    if (entry?.source === "windows_start_apps" && target === entry.path) {
      return `${entry.sourceLabel}: ${target}`;
    }
    return target;
  };
  const selectedExecutablePath =
    selectedAgentPreference?.executable?.path ??
    selectedAgentPreference?.executablePath ??
    (selectedAgentIsDirectOnly
      ? desktopAppPathForAgent(agentId)
      : agentExecutable?.selected?.path) ??
    selectedAgent.pty_command;
  const selectedExecutableLabel = selectedAgentIsDirectOnly
    ? desktopAppLaunchTargetLabel(agentId, selectedExecutablePath)
    : selectedExecutablePath;
  const showClaudeDesktopDeveloperModeHint =
    agentId === "claude-desktop" && profileChoice?.kind === "profile";

  return (
    <TooltipProvider>
      <div className="flex min-h-0 flex-1">
        <aside className="w-[74px] shrink-0 border-r border-border bg-card/50 px-2 py-3">
          <div className="flex flex-col gap-2">
            {agents.map((agent) => (
              <AgentRailButton
                key={agent.id}
                agent={agent}
                active={agent.id === agentId}
                isDefault={viewPrefs.defaultAgent === agent.id}
                onClick={() => void chooseAgent(agent.id)}
              />
            ))}
          </div>
        </aside>

        <main className="flex min-w-0 flex-1 flex-col">
          <div className="flex min-h-0 flex-1 flex-col">
            <header className="bg-card/20 p-3">
              <div className="grid grid-cols-4 items-stretch gap-2">
                <div className="col-span-3 overflow-visible rounded-md border border-border bg-card p-3 shadow-sm">
                  <AgentSummaryHeader
                    agentId={agentId}
                    agentLabelText={selectedAgent.display_name}
                    action={undefined}
                  >
                    <>
                      <SelectorPopup
                        id="profile"
                        openSelector={openSelector}
                        onOpenChange={setOpenSelector}
                        widthClassName="w-max min-w-[340px] max-w-[min(680px,calc(100vw-1rem))]"
                        widthPx={680}
                        trigger={
                          <button
                            type="button"
                            className={`mt-0.5 flex max-w-[520px] min-w-0 cursor-pointer items-center gap-1 rounded-sm text-left text-[12px] leading-5 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${
                              openSelector === "profile"
                                ? "text-primary"
                                : "text-muted-foreground hover:text-foreground"
                            }`}
                            onClick={() =>
                              setOpenSelector(
                                openSelector === "profile" ? null : "profile",
                              )
                            }
                          >
                            <span className="min-w-0 truncate font-semibold text-foreground">
                              {selectedProfileSummary.title}
                            </span>
                            <span className="min-w-0 truncate text-muted-foreground">
                              <span className="px-0.5">·</span>
                              {selectedProfileSummary.route}
                            </span>
                          </button>
                        }
                      >
                        <ProfileInfoPanel
                          agentId={agentId}
                          prefs={viewPrefs}
                          profile={selectedProfile}
                          summary={selectedProfileSummary}
                        />
                      </SelectorPopup>
                      <div className="mt-1 flex max-w-[680px] min-w-0 items-center gap-1.5 text-[11px] leading-4 text-muted-foreground">
                        {selectedAgentIsDirectOnly ? (
                          <Monitor className="h-3.5 w-3.5 shrink-0" />
                        ) : (
                          <Terminal className="h-3.5 w-3.5 shrink-0" />
                        )}
                        <span
                          className="min-w-0 truncate font-mono [font-variant-ligatures:none]"
                          title={selectedExecutableLabel}
                        >
                          {!selectedAgentIsDirectOnly && agentExecutableLoading
                            ? t("Checking path")
                            : selectedExecutableLabel}
                        </span>
                        <TooltipButton
                          type="button"
                          variant="ghost"
                          size="icon-xs"
                          disabled={busy}
                          aria-label={t("Change agent path")}
                          title={t("Change agent path")}
                          onClick={() => openAgentPathDialog(selectedAgent)}
                          className="h-5 w-5 shrink-0 text-muted-foreground hover:text-foreground"
                        >
                          <Pencil className="h-3 w-3" />
                        </TooltipButton>
                      </div>
                      {showClaudeDesktopDeveloperModeHint && (
                        <div className="mt-1 space-y-0.5">
                          <p className="max-w-[520px] text-[11px] leading-4 text-muted-foreground">
                            {t(
                              "For Anthropic profiles, Claude Desktop opens the local bridge automatically on launch.",
                            )}
                          </p>
                          <p className="max-w-[640px] text-[11px] leading-4 text-muted-foreground">
                            {t(
                              "Claude Desktop profile launch requires Developer Mode. Enable it in Claude Desktop: Help -> Troubleshooting -> Enable Developer Mode.",
                            )}
                          </p>
                        </div>
                      )}
                    </>
                  </AgentSummaryHeader>
                  {showLaunchControls && (
                    <div className="mt-3 flex min-w-0 flex-wrap items-center gap-2">
                      <SelectorPopup
                        id="workspace"
                        openSelector={openSelector}
                        onOpenChange={setOpenSelector}
                        widthClassName="w-[min(400px,calc(100vw-1rem))]"
                        widthPx={400}
                        trigger={
                          <LaunchSummaryPill
                            active={openSelector === "workspace"}
                            chevron
                            className="w-[250px]"
                            icon={<FolderOpen className="h-4 w-4" />}
                            onClick={() =>
                              setOpenSelector(
                                openSelector === "workspace"
                                  ? null
                                  : "workspace",
                              )
                            }
                            label={t("Workspace")}
                            title={selectedWorkspace.label}
                            detail={
                              workspacesLoading ? t("Loading…") : undefined
                            }
                          />
                        }
                      >
                        <WorkspacePanel
                          prefs={viewPrefs}
                          loading={workspacesLoading}
                          onSelect={(path) => {
                            setOpenSelector(null);
                            void chooseWorkspace(path);
                          }}
                          onDelete={(path, label) =>
                            void removeWorkspace(path, label)
                          }
                          onReorder={(fromPath, toPath) =>
                            void reorderWorkspace(fromPath, toPath)
                          }
                          onCreate={() => {
                            setOpenSelector(null);
                            void chooseFolder();
                          }}
                          busy={busy}
                        />
                      </SelectorPopup>
                    </div>
                  )}
                </div>
                <div className="col-span-1 flex">
                  <TooltipButton
                    type="button"
                    disabled={busy || !selectionLaunchable}
                    disabledReason={launchDisabledReason}
                    onClick={() => void launchSelected()}
                    size="lg"
                    className="h-full min-h-[115px] w-full justify-center gap-4 rounded-md text-[28px] font-semibold tracking-[0.12em] shadow-md shadow-primary/15 transition-none"
                  >
                    <Rocket className="size-8" />
                    {t("LAUNCH")}
                  </TooltipButton>
                </div>
              </div>
            </header>

            <section className="min-h-0 flex-1 overflow-y-auto px-3 pb-3">
              <ProfilePanel
                agentId={agentId}
                prefs={viewPrefs}
                selected={profileChoice}
                profiles={profileOptions}
                onSelect={(choice) => void chooseProfileChoice(choice)}
                onSelectApiType={(profile, apiType) =>
                  void chooseProfileApiType(profile, apiType)
                }
                onMakeDefault={makeDefault}
                onEditProfile={onEditProfile}
                onDuplicateProfile={(profile) => void duplicateProfile(profile)}
                onConnectionSettings={onConnectionSettings}
                onDeleteProfile={(profile) => void removeProfile(profile)}
                onReorderProfile={(fromId, toId) =>
                  void reorderProfile(fromId, toId)
                }
                onNewProfile={onNewProfile}
                busy={busy}
              />
            </section>
          </div>
        </main>
      </div>
      <AgentExecutablePathDialog
        agent={pathAgent}
        preference={
          pathAgent ? viewPrefs.agentPreferences[pathAgent.id] : undefined
        }
        executableResolution={pathAgent?.id === agentId ? agentExecutable : null}
        executableLoading={
          pathAgent?.id === agentId ? agentExecutableLoading : false
        }
        fallbackExecutablePath={
          pathAgent?.direct_only
            ? (desktopAppPathForAgent(pathAgent.id) ?? pathAgent.pty_command)
            : undefined
        }
        busy={busy}
        onClose={() => setPathAgent(null)}
        onSaveExecutablePath={saveAgentExecutablePath}
        onRefreshExecutableResolution={() =>
          pathAgent ? refreshAgentExecutable(pathAgent.id) : Promise.resolve()
        }
        onCheckLatest={checkAgentExecutableLatest}
        onUpdateAgent={updateAgentExecutable}
      />
    </TooltipProvider>
  );
}
