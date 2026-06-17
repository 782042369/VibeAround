import type { AgentId } from "../constants";
import type { Settings } from "../types";

export interface BuildSettingsInput {
  settings: Settings;
  enabledAgents: Set<AgentId>;
}

export function buildSettings({
  settings,
  enabledAgents,
}: BuildSettingsInput): Settings {
  const result: Settings = {
    ...settings,
    enabled_agents: Array.from(enabledAgents),
  };

  delete result.default_workspace;
  delete result.default_agent;
  delete result.default_profiles;
  delete result.channels;
  delete result.tunnel;

  return result;
}
