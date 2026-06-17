// Resource types returned by Tauri commands.
export interface AgentSummary {
  id: string;
  display_name: string;
  description: string;
  install_type?: "npm" | "script" | "path";
  direct_only?: boolean;
  download_url?: string | null;
}

export interface Settings {
  onboarded?: boolean;
  workspaces?: string[];
  default_agent?: string;
  default_profiles?: Record<string, string>;
  enabled_agents?: string[];
  integrations?: {
    mcp_auto_install?: boolean;
    skill_auto_install?: boolean;
  };
  im_agent?: {
    auto_continue_last_session?: boolean;
  };
  proxy?: {
    enabled?: boolean;
    http_proxy?: string;
    no_proxy?: string;
  };
  api_bridge?: {
    retry_429?: {
      enabled?: boolean;
      max_retries?: number | null;
      delay_seconds?: number;
    };
  };
  startkit?: {
    source?: string;
    toolchain_mode?: ToolchainMode | string;
    shell_path?: boolean;
  };
  [key: string]: unknown;
}

export type StartkitStatus =
  | "pending"
  | "running"
  | "ok"
  | "missing"
  | "outdated"
  | "broken"
  | "needs_config"
  | "blocked"
  | "error"
  | "skipped";

export interface StartkitChoices {
  agents: string[];
  source: string;
  toolchainMode: ToolchainMode;
}

export type ToolchainMode = "system" | "managed";

export interface StartkitSource {
  label: string;
  node_index: string;
  node_dist: string;
  npm_registry: string;
}

export interface StartkitItemSummary {
  id: string;
  label: string;
  group: string;
  category: string;
  description?: string;
  severity?: string;
  kind?: string;
  managed: boolean;
  hasRepair: boolean;
  secret: boolean;
  settingsKey?: string;
}

export interface StartkitManifestSummary {
  id: string;
  name: string;
  schema: number;
  version: string;
  sources: Record<string, StartkitSource>;
  items: StartkitItemSummary[];
}

export interface StartkitPlan {
  platform: string;
  source: string;
  itemIds: string[];
  items: StartkitItemSummary[];
}

export interface StartkitItemReport {
  id: string;
  label: string;
  group: string;
  category: string;
  status: StartkitStatus;
  severity?: string;
  version?: string;
  latestVersion?: string;
  path?: string;
  message?: string;
  actions: string[];
  manualCommand?: string;
  manualUrl?: string;
  secret: boolean;
  settingsKey?: string;
}

export interface StartkitScanReport {
  plan: StartkitPlan;
  reports: StartkitItemReport[];
}

export interface StartkitProgressEvent {
  id: string;
  label: string;
  status: StartkitStatus;
  message?: string;
  report?: StartkitItemReport;
}

export interface StartkitCompleteEvent {
  status: string;
}
