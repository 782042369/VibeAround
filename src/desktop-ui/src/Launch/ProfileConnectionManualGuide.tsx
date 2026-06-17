import { Check, Copy } from "lucide-react";
import { useI18n } from "@va/i18n";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { API_BASE } from "@/lib/api";
import type { ConnectionAgentId, ProfileSummary } from "./types";

export const PLACEHOLDER_API_KEY = "anything-non-empty";

export interface ManualBridgeConfig {
  baseUrl: string;
  model: string;
  copyKey: string;
}

export interface ManualSetting {
  agentId: ConnectionAgentId;
  agentLabel: string;
  copyKey: string;
  filePath: string;
  profileName?: string;
  snippet: string;
}

export function manualBridgeConfig(
  profileId: string,
  agentId: ConnectionAgentId,
  clientApiType: string,
  targetApiType: string,
  model: string | undefined,
): ManualBridgeConfig {
  const path = [
    "local-api",
    encodeURIComponent(profileId),
    encodeURIComponent(`${agentId}-${clientApiType}`),
    encodeURIComponent(targetApiType),
  ].join("/");
  const versionSuffix = clientApiType === "anthropic" ? "" : "/v1";
  return {
    baseUrl: `${API_BASE}/${path}${versionSuffix}`,
    model: model ?? "",
    copyKey: `${agentId}:${clientApiType}:${targetApiType}:base-url`,
  };
}

export function buildManualSetting(
  profile: ProfileSummary,
  agentId: ConnectionAgentId,
  agentLabel: string,
  _clientApiType: string,
  _targetApiType: string,
  manualConfig: ManualBridgeConfig,
): ManualSetting {
  const model = manualConfig.model || "<model-id>";
  if (agentId === "codex") {
    const providerName = codexProviderName(profile.label);
    return {
      agentId,
      agentLabel,
      copyKey: `${manualConfig.copyKey}:codex-config`,
      filePath: "~/.codex/config.toml 和 ~/.codex/auth.json",
      profileName: providerName,
      snippet: [
        `# ~/.codex/config.toml`,
        "",
        `model = ${tomlString(model)}`,
        `model_provider = ${tomlString(providerName)}`,
        `model_reasoning_effort = "medium"`,
        "",
        `[model_providers.${tomlKey(providerName)}]`,
        `name = ${tomlString(providerName)}`,
        `base_url = ${tomlString(manualConfig.baseUrl)}`,
        `wire_api = "responses"`,
        `requires_openai_auth = true`,
        "",
        `# ~/.codex/auth.json`,
        "",
        `{`,
        `  "OPENAI_API_KEY": "<你的中转站 key>"`,
        `}`,
        "",
      ].join("\n"),
    };
  }

  const claudeEnv: Record<string, string> = {
    ANTHROPIC_API_KEY: PLACEHOLDER_API_KEY,
    ANTHROPIC_AUTH_TOKEN: PLACEHOLDER_API_KEY,
    ANTHROPIC_BASE_URL: manualConfig.baseUrl,
    ANTHROPIC_MODEL: model,
  };

  return {
    agentId,
    agentLabel,
    copyKey: `${manualConfig.copyKey}:claude-settings`,
    filePath: "~/.claude/settings.json",
    snippet: `"env": ${JSON.stringify(claudeEnv, null, 2)}`,
  };
}

export function ManualSettingDialog({
  setting,
  copiedKey,
  onCopy,
  onClose,
}: {
  setting: ManualSetting;
  copiedKey: string | null;
  onCopy: (key: string, value: string) => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const isCodex = setting.agentId === "codex";

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="!flex max-h-[calc(100vh-64px)] w-[min(700px,calc(100vw-32px))] max-w-[calc(100vw-32px)] flex-col overflow-hidden p-0 sm:max-w-[min(700px,calc(100vw-32px))]">
        <DialogHeader className="shrink-0 px-6 pt-6 pr-12">
          <DialogTitle>
            {t("{{agent}} manual setting", { agent: setting.agentLabel })}
          </DialogTitle>
          <DialogDescription>
            {t("Copy this snippet into the CLI config file yourself. VibeWbz does not edit the file automatically.")}
          </DialogDescription>
        </DialogHeader>

        <div className="grid min-h-0 flex-1 gap-3 overflow-y-auto px-6 pb-6 [scrollbar-gutter:stable]">
          <div className="grid gap-2 rounded-md border border-border/70 bg-muted/25 p-3 text-[12px]">
            <ConfigInfoRow label={t("Configuration file")} value={setting.filePath} />
          </div>

          <div className="rounded-md border border-border/70 p-3">
            <div className="text-[12px] font-medium">{t("How to modify")}</div>
            <ol className="mt-2 space-y-1.5 pl-4 text-[12px] leading-relaxed text-muted-foreground">
              {isCodex ? (
                <>
                  <li>{t("Put the config.toml part into ~/.codex/config.toml.")}</li>
                  <li>{t("Put the auth.json part into ~/.codex/auth.json, in the same folder as config.toml.")}</li>
                  <li>{t("The top-level model and provider lines make Codex use this gateway by default.")}</li>
                </>
              ) : (
                <>
                  <li>{t("Paste this property inside the root JSON object of Claude settings.")}</li>
                  <li>{t("If env already exists, merge these keys into the existing env object instead of creating another env block.")}</li>
                  <li>{t("If Claude keeps using account login instead of this env block, run claude auth logout first.")}</li>
                </>
              )}
            </ol>
          </div>

          <ConfigSnippetBlock
            title={
              isCodex
                ? t("Codex config snippet")
                : t("Config snippet")
            }
            snippet={setting.snippet}
            copied={copiedKey === setting.copyKey}
            onCopy={() => onCopy(setting.copyKey, setting.snippet)}
          />
        </div>
      </DialogContent>
    </Dialog>
  );
}

function ConfigSnippetBlock({
  title,
  snippet,
  copied,
  onCopy,
}: {
  title: string;
  snippet: string;
  copied: boolean;
  onCopy: () => void;
}) {
  const { t } = useI18n();

  return (
    <div
      className={`overflow-hidden rounded-md border ${
        copied
          ? "border-primary/60 bg-primary/10"
          : "border-primary/30 bg-primary/5"
      }`}
    >
      <div className="flex items-center justify-between gap-2 border-b border-primary/20 px-3 py-2">
        <div className="text-[12px] font-medium text-primary">{title}</div>
        <Button
          type="button"
          variant="ghost"
          size="xs"
          className="h-6 gap-1.5 px-2 text-[11px] font-medium text-primary hover:bg-primary/10 hover:text-primary"
          onClick={onCopy}
        >
          {copied ? <Check className="h-3 w-3" /> : <Copy className="h-3 w-3" />}
          {copied ? t("Copied") : t("Copy config")}
        </Button>
      </div>
      <pre className="max-h-[280px] overflow-auto whitespace-pre-wrap break-words px-3 py-2 font-mono text-[11px] leading-relaxed text-foreground">
        {snippet}
      </pre>
    </div>
  );
}

function ConfigInfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex flex-wrap items-center gap-3">
      <div className="shrink-0 text-muted-foreground">{label}</div>
      <div className="break-all font-mono text-foreground">{value}</div>
    </div>
  );
}

export function ManualValueRow({
  label,
  value,
  copied,
  onCopy,
}: {
  label: string;
  value: string;
  copied: boolean;
  onCopy: () => void;
}) {
  const { t } = useI18n();

  return (
    <div className="grid grid-cols-[56px_minmax(0,1fr)] items-center gap-1">
      <div className="text-[11px] text-muted-foreground">{label}</div>
      <button
        type="button"
        className={`group flex min-w-0 cursor-pointer items-center rounded px-0.5 py-0 text-left font-mono text-[11px] leading-5 transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring ${
          copied
            ? "bg-primary/10 text-primary"
            : "text-foreground hover:bg-primary/5 hover:text-primary"
        }`}
        onClick={onCopy}
        title={value}
      >
        <span className="min-w-0 flex-1 truncate">{value}</span>
        {copied && (
          <span className="ml-1.5 inline-flex shrink-0 items-center gap-1 text-[10px] font-sans">
            <Check className="h-3 w-3" />
            {t("Copied")}
          </span>
        )}
      </button>
    </div>
  );
}

function codexProviderName(label: string): string {
  return label.trim() || "VibeWbz Gateway";
}

function tomlKey(value: string): string {
  return /^[A-Za-z0-9_-]+$/.test(value) ? value : tomlString(value);
}

function tomlString(value: string): string {
  return JSON.stringify(value);
}
