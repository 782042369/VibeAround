import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

import {
  hydrateAgents,
  hydrateStartkitPrefs,
} from "../lib/hydrateSettings";
import type { AgentId } from "../constants";
import type {
  AgentSummary,
  Settings,
  StartkitManifestSummary,
} from "../types";

const AGENT_DISPLAY_ORDER = [
  "claude",
  "codex",
  "claude-desktop",
  "codex-desktop",
];

export function useOnboardingInitialLoad({
  setSettings,
  setLoaded,
  setManifest,
  setAgents,
  setDownloadSource,
  setEnabledAgents,
}: {
  setSettings: (value: Settings) => void;
  setLoaded: (value: boolean) => void;
  setManifest: (value: StartkitManifestSummary) => void;
  setAgents: (value: AgentSummary[]) => void;
  setDownloadSource: (value: string) => void;
  setEnabledAgents: (value: Set<AgentId>) => void;
}) {
  useEffect(() => {
    Promise.all([
      invoke<Settings>("get_settings"),
      invoke<AgentSummary[]>("list_agents"),
      invoke<StartkitManifestSummary>("startkit_manifest"),
    ])
      .then(
        ([
          loadedSettings,
          agentDefs,
          startkitManifest,
        ]) => {
          const orderedAgents = orderAgents(agentDefs);
          setSettings(loadedSettings);
          setAgents(orderedAgents);
          setManifest(startkitManifest);

          hydrateStartkitPrefs(loadedSettings, {
            setDownloadSource,
          });
          hydrateAgents(loadedSettings, orderedAgents, setEnabledAgents);

          setLoaded(true);
        },
      )
      .catch((error) => {
        console.error("failed to load onboarding data", error);
        setLoaded(true);
      });
  }, [
    setAgents,
    setDownloadSource,
    setEnabledAgents,
    setLoaded,
    setManifest,
    setSettings,
  ]);
}

function orderAgents(agentDefs: AgentSummary[]): AgentSummary[] {
  const rank = new Map(AGENT_DISPLAY_ORDER.map((id, index) => [id, index]));
  return agentDefs
    .filter((agent) => rank.has(agent.id))
    .sort((a, b) => (rank.get(a.id) ?? 999) - (rank.get(b.id) ?? 999));
}
