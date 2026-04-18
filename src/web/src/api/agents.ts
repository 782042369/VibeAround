/**
 * Agents API: fetch enabled agents from backend.
 */

import { browserBaseUrl } from "@va/client";
import type { AgentInfo } from "@va/generated/AgentInfo";
import type { AgentsConfig } from "@va/generated/AgentsConfig";

export type { AgentInfo, AgentsConfig };

export async function getAgents(): Promise<AgentsConfig> {
  const res = await fetch(`${browserBaseUrl()}/api/agents`);
  if (!res.ok) throw new Error(`GET /api/agents: ${res.status}`);
  return res.json();
}
