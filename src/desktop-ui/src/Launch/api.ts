import { invoke } from "@tauri-apps/api/core";

import type {
  CatalogEntry,
  AgentLaunchPreference,
  ConnectionAgentId,
  ProfileDef,
  ProfileDraft,
  ProfileConnectionPreference,
  ProfileConnections,
  ProfileSummary,
} from "./types";
import type { Settings } from "../Onboarding/types";

export function listProfiles(): Promise<ProfileSummary[]> {
  return invoke<ProfileSummary[]>("profiles_list");
}

export function getProfile(id: string): Promise<ProfileDef> {
  return invoke<ProfileDef>("profiles_get", { id });
}

export function upsertProfile(profile: ProfileDef): Promise<void> {
  return invoke<void>("profiles_upsert", { profile });
}

export function createProfile(draft: ProfileDraft): Promise<ProfileDef> {
  return invoke<ProfileDef>("profiles_create", { draft });
}

export function deleteProfile(id: string): Promise<void> {
  return invoke<void>("profiles_delete", { id });
}

export function reorderProfiles(profileIds: string[]): Promise<void> {
  return invoke<void>("profiles_reorder", { profileIds });
}

export function launchProfile(id: string, launchTarget: string): Promise<void> {
  return invoke<void>("profiles_launch", { id, launchTarget });
}

export function launchDefault(): Promise<void> {
  return invoke<void>("profiles_launch_default");
}

/** Direct launch — no env, CLI uses whatever global OAuth the user has. */
export function launchDirect(agentId: string): Promise<void> {
  return invoke<void>("profiles_launch_direct", { agentId });
}

export interface GoogleOAuthStatus {
  signedIn: boolean;
  path: string;
  expiresAt?: number | null;
}

export function googleOAuthStatus(): Promise<GoogleOAuthStatus> {
  return invoke<GoogleOAuthStatus>("profiles_google_oauth_status");
}

export function googleOAuthLogin(): Promise<GoogleOAuthStatus> {
  return invoke<GoogleOAuthStatus>("profiles_google_oauth_login");
}

export interface AgentSummary {
  id: string;
  display_name: string;
  description: string;
  install_type: string | null;
  pty_command: string;
  direct_only: boolean;
  download_url?: string | null;
}

/** Reuses the onboarding command that returns all CLIs in agents.json. */
export function listAgents(): Promise<AgentSummary[]> {
  return invoke<AgentSummary[]>("list_agents");
}

export function rescanAgentEntries(): Promise<unknown> {
  return invoke("rescan_agent_entries");
}

export interface AgentExecutableCandidate {
  path: string;
  realpath?: string | null;
  version?: string | null;
  latestVersion?: string | null;
  updateAvailable?: boolean | null;
  source: string;
  sourceLabel: string;
  rank: number;
  selected: boolean;
  updateCommand?: string | null;
}

export interface AgentExecutableResolution {
  agentId: string;
  configuredPath?: string | null;
  selected?: AgentExecutableCandidate | null;
  candidates: AgentExecutableCandidate[];
}

export interface AgentExecutableLatest {
  path: string;
  latestVersion?: string | null;
  updateAvailable?: boolean | null;
}

export function getAgentExecutableResolution(
  agentId: string,
): Promise<AgentExecutableResolution> {
  return invoke<AgentExecutableResolution>("launcher_agent_executable_resolution", {
    agentId,
  });
}

export function getAgentExecutableLatest(
  agentId: string,
  executablePath: string,
): Promise<AgentExecutableLatest> {
  return invoke<AgentExecutableLatest>("launcher_agent_executable_latest", {
    agentId,
    executablePath,
  });
}

export function updateLauncherAgent(
  agentId: string,
  executablePath?: string | null,
): Promise<void> {
  return invoke<void>("launcher_update_agent", {
    agentId,
    executablePath: executablePath ?? null,
  });
}

export interface DesktopAppDetectionFile {
  apps: Record<string, DesktopAppDetection>;
}

export interface DesktopAppDetection {
  installed: boolean;
  launchCommand: string;
  entry?: {
    appName: string;
    path: string;
    source: string;
    sourceLabel: string;
  } | null;
}

export function rescanDesktopAppEntries(): Promise<DesktopAppDetectionFile> {
  return invoke<DesktopAppDetectionFile>("rescan_desktop_app_entries");
}

export function getDesktopAppEntries(): Promise<DesktopAppDetectionFile | null> {
  return invoke<DesktopAppDetectionFile | null>("get_desktop_app_entries");
}

export interface LauncherPreferences {
  workspace: string;
  workspaceOptions: WorkspaceOption[];
  selectedAgent: string;
  agentPreferences: Record<string, AgentLaunchPreference>;
  defaultAgent: string;
  defaultProfileId?: string | null;
  enabledAgents: string[];
  defaultProfiles: Record<string, string>;
  profileConnections: ProfileConnections;
}

export interface WorkspaceOption {
  path: string;
  label: string;
  detail: string;
  kind: string;
  isDefault: boolean;
}

export function getLauncherPreferences(): Promise<LauncherPreferences> {
  return invoke<LauncherPreferences>("launcher_get_preferences");
}

export function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

export function listLauncherWorkspaces(agentId?: string): Promise<WorkspaceOption[]> {
  return invoke<WorkspaceOption[]>("launcher_list_workspaces", {
    agentId: agentId ?? null,
  });
}

export function setLauncherWorkspace(
  workspacePath: string,
  agentId?: string,
): Promise<void> {
  return invoke<void>("launcher_set_workspace", {
    workspacePath,
    agentId: agentId ?? null,
  });
}

export function removeLauncherWorkspace(workspacePath: string): Promise<void> {
  return invoke<void>("launcher_remove_workspace", { workspacePath });
}

export function reorderLauncherWorkspaces(
  workspacePaths: string[],
): Promise<void> {
  return invoke<void>("launcher_reorder_workspaces", { workspacePaths });
}

export function setProfileConnection(
  profileId: string,
  agentId: ConnectionAgentId,
  preference: ProfileConnectionPreference,
): Promise<void> {
  return invoke<void>("launcher_set_profile_connection", {
    profileId,
    agentId,
    preference,
  });
}

export function setLauncherDefault(
  agentId: string,
  profileId: string | null,
): Promise<void> {
  return invoke<void>("launcher_set_default", { agentId, profileId });
}

export function setLauncherAgentProfile(
  agentId: string,
  profileId: string | null,
): Promise<void> {
  return invoke<void>("launcher_set_agent_profile", { agentId, profileId });
}

export function setLauncherAgentExecutablePath(
  agentId: string,
  executablePath: string | null,
): Promise<void> {
  return invoke<void>("launcher_set_agent_executable_path", {
    agentId,
    executablePath,
  });
}

export function setLauncherSelectedAgent(agentId: string): Promise<void> {
  return invoke<void>("launcher_set_selected_agent", { agentId });
}

export function listCatalog(): Promise<CatalogEntry[]> {
  return invoke<CatalogEntry[]>("profiles_catalog");
}

export type ProfileModelOption = unknown;

export function fetchProfileModels(
  baseUrl: string,
  apiKey: string,
): Promise<ProfileModelOption[]> {
  return invoke<ProfileModelOption[]>("profiles_fetch_models", {
    request: { baseUrl, apiKey },
  });
}
