import type { AgentId } from "../constants";
import type {
  AgentSummary,
  Settings,
  ToolchainMode,
} from "../types";

const DEFAULT_ENABLED_AGENT_IDS = new Set<AgentId>([
  "claude",
  "codex",
  "claude-desktop",
  "codex-desktop",
]);

export function hydrateStartkitPrefs(
  loadedSettings: Settings,
  setters: {
    setDownloadSource: (value: string) => void;
    setToolchainMode: (value: ToolchainMode) => void;
  },
) {
  if (loadedSettings.startkit?.source) {
    setters.setDownloadSource(loadedSettings.startkit.source);
  }
  if (
    loadedSettings.startkit?.toolchain_mode === "system" ||
    loadedSettings.startkit?.toolchain_mode === "managed"
  ) {
    setters.setToolchainMode(loadedSettings.startkit.toolchain_mode);
  }
}

export function hydrateAgents(
  loadedSettings: Settings,
  orderedAgents: AgentSummary[],
  setEnabledAgents: (value: Set<AgentId>) => void,
) {
  if (Array.isArray(loadedSettings.enabled_agents)) {
    setEnabledAgents(new Set(loadedSettings.enabled_agents as AgentId[]));
    return;
  }
  setEnabledAgents(
    new Set(
      orderedAgents
        .map((agent) => agent.id)
        .filter((id) => DEFAULT_ENABLED_AGENT_IDS.has(id)),
    ),
  );
}
