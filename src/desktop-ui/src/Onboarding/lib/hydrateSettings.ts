import type { AgentId } from "../constants";
import type {
  AgentSummary,
  Settings,
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
  },
) {
  if (loadedSettings.startkit?.source) {
    setters.setDownloadSource(loadedSettings.startkit.source);
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
