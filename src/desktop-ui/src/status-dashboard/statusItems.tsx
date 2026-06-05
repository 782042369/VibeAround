import { ExternalLink, Play, RotateCw, Square, X } from "lucide-react";

import { Button } from "@/components/ui/button";
import { openDashboardUrl } from "@/lib/api";
import type { AgentRuntime } from "../hooks/useAgentsRuntime";
import type { ChannelRuntime } from "../hooks/useChannelsState";
import type { TunnelRuntime } from "../hooks/useTunnelsState";
import {
  agentDisplayName,
  basename,
  capitalize,
  channelDisplayName,
  channelPresentation,
  formatDuration,
  shortId,
  tunnelDetail,
  tunnelPresentation,
} from "./presentation";
import { AgentIconBadge, ServiceIconBadge } from "./serviceIcon";
import type { RuntimeStatusItem } from "./statusCard";
import type { Tone, Translate } from "./types";

export function buildTunnelStatusItems({
  tunnels,
  kill,
  t,
}: {
  tunnels: TunnelRuntime[];
  kill: (provider: string) => unknown;
  t: Translate;
}): RuntimeStatusItem[] {
  return tunnels.map((tunnel) => {
    const presentation = tunnelPresentation(tunnel.status, t);
    const name = t("{{provider}} tunnel", {
      provider: capitalize(tunnel.provider),
    });

    return {
      id: tunnel.provider,
      kind: "tunnel",
      name,
      status: presentation.label,
      tone: presentation.tone,
      icon: (
        <ServiceIconBadge
          id={tunnel.provider}
          kind="tunnel"
          tone={presentation.tone}
        />
      ),
      details: [
        { label: t("Type"), value: t("Remote access") },
        { label: t("Name"), value: name },
        { label: t("Provider"), value: tunnel.provider },
        { label: t("Status"), value: presentation.label },
        {
          label: t("URL"),
          value: tunnel.url ? (
            <button
              type="button"
              className="text-primary underline-offset-2 hover:underline"
              onClick={() => void openDashboardUrl(tunnel.url!)}
            >
              {tunnel.url}
            </button>
          ) : (
            t("Not available")
          ),
        },
        {
          label: t("Uptime"),
          value:
            tunnel.uptime_secs > 0
              ? formatDuration(tunnel.uptime_secs)
              : t("Not available"),
        },
        {
          label: t("Message"),
          value: tunnelDetail(tunnel.status) ?? t("No issues reported"),
        },
      ],
      actions: (
        <>
          {tunnel.url && (
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => void openDashboardUrl(tunnel.url!)}
            >
              <ExternalLink className="h-3.5 w-3.5" />
              {t("Open")}
            </Button>
          )}
          {tunnel.status.state === "running" && (
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => kill(tunnel.provider)}
            >
              <X className="h-3.5 w-3.5" />
              {t("Stop")}
            </Button>
          )}
        </>
      ),
    };
  });
}

export function buildChannelStatusItems({
  channels,
  start,
  stop,
  restart,
  t,
}: {
  channels: ChannelRuntime[];
  start: (kind: string) => unknown;
  stop: (kind: string) => unknown;
  restart: (kind: string) => unknown;
  t: Translate;
}): RuntimeStatusItem[] {
  return channels.map((channel) => {
    const presentation = channelPresentation(channel.status, t);
    const name = channelDisplayName(channel.kind);
    const running = channel.status === "running" || channel.status === "spawning";

    return {
      id: channel.kind,
      kind: "channel",
      name,
      status: presentation.label,
      tone: presentation.tone,
      icon: (
        <ServiceIconBadge
          id={channel.kind}
          kind="channel"
          tone={presentation.tone}
        />
      ),
      details: [
        { label: t("Type"), value: t("Messaging app") },
        { label: t("Name"), value: name },
        { label: t("Plugin version"), value: t("Not reported") },
        { label: t("Status"), value: presentation.label },
        { label: t("Crashes"), value: String(channel.crash_count) },
        {
          label: t("Last seen"),
          value:
            channel.last_seen_age_secs > 0
              ? t("{{duration}} ago", {
                  duration: formatDuration(channel.last_seen_age_secs),
                })
              : t("Just now"),
        },
        {
          label: t("Restart in"),
          value:
            channel.restart_in_secs > 0
              ? formatDuration(channel.restart_in_secs)
              : t("Not scheduled"),
        },
        { label: t("Message"), value: channel.reason ?? t("No issues reported") },
      ],
      actions: (
        <>
          {running ? (
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => stop(channel.kind)}
            >
              <Square className="h-3.5 w-3.5" />
              {t("Stop")}
            </Button>
          ) : (
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => start(channel.kind)}
            >
              <Play className="h-3.5 w-3.5" />
              {t("Start")}
            </Button>
          )}
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => restart(channel.kind)}
          >
            <RotateCw className="h-3.5 w-3.5" />
            {t("Restart")}
          </Button>
        </>
      ),
    };
  });
}

export function buildAgentStatusItems({
  agents,
  kill,
  t,
}: {
  agents: AgentRuntime[];
  kill: (routeKey: string) => unknown;
  t: Translate;
}): RuntimeStatusItem[] {
  return agents.map((agent) => {
    const failed = Boolean(agent.failed);
    const status = failed ? t("Failed") : agent.busy ? t("Busy") : t("Idle");
    const tone: Tone = failed ? "danger" : agent.busy ? "busy" : "good";
    const name = agentDisplayName(agent, t);

    return {
      id: agent.route_key,
      kind: "agent",
      name,
      status,
      tone,
      icon: (
        <AgentIconBadge
          cliKind={agent.cli_kind}
          label={name}
          tone={tone}
        />
      ),
      details: [
        { label: t("Type"), value: t("Coding Agent") },
        { label: t("Name"), value: name },
        { label: t("Status"), value: status },
        { label: t("CLI"), value: agent.cli_kind ?? t("Not reported") },
        { label: t("Version"), value: agent.agent_version ?? t("Not reported") },
        {
          label: t("Workspace"),
          value: agent.workspace ? basename(agent.workspace) : t("Not reported"),
        },
        {
          label: t("Session"),
          value: agent.session_id ? shortId(agent.session_id) : t("Not reported"),
        },
        { label: t("Route"), value: agent.route_key },
        {
          label: t("Subagents"),
          value: String(agent.subagents.length + agent.multi_agent_turns.length),
        },
        { label: t("Message"), value: agent.failed ?? t("No issues reported") },
      ],
      actions: !failed ? (
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => kill(agent.route_key)}
        >
          <X className="h-3.5 w-3.5" />
          {t("Stop")}
        </Button>
      ) : null,
    };
  });
}
