import { useState } from "react";
import { useI18n } from "@va/i18n";
import { ExternalLink, Eye, EyeOff, KeyRound } from "lucide-react";

import { Button } from "@/components/ui/button";
import { openExternalUrl } from "@/lib/api";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import {
  GATEWAY_BASE_URL,
  GATEWAY_PROFILE_LABEL,
  gatewayProfileDraft,
} from "./gatewayProfile";
import type {
  CatalogEntry,
  ProfileDef,
  ProfileDraft,
} from "./types";

const GATEWAY_HOME_URL = "https://ai.939593.xyz";
const GATEWAY_TOPUP_URL = "https://ai.939593.xyz/console/topup";
const GATEWAY_TOKEN_URL = "https://ai.939593.xyz/console/token";

export type ProfileFormSubmit =
  | { type: "create"; draft: ProfileDraft }
  | { type: "update"; profile: ProfileDef };

interface Props {
  catalog: CatalogEntry[];
  /** Set when editing -- locks step 1 and prefills step 2. */
  initial?: ProfileDef | null;
  onClose: () => void;
  onSave: (submit: ProfileFormSubmit) => Promise<void>;
}

export function ProfileFormDialog({
  initial,
  onClose,
  onSave,
}: Props) {
  const { t } = useI18n();
  const editing = !!initial;
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [gatewayLabel, setGatewayLabel] = useState(
    initial?.label ?? GATEWAY_PROFILE_LABEL,
  );
  const [gatewayBaseUrl, setGatewayBaseUrl] = useState(
    initial?.overrides.anthropic?.base_url ??
      initial?.overrides["openai-responses"]?.base_url ??
      GATEWAY_BASE_URL,
  );
  const [gatewayKey, setGatewayKey] = useState(initial?.credentials.api_key ?? "");
  const [gatewayKeyVisible, setGatewayKeyVisible] = useState(false);

  async function handleSaveGateway() {
    setError(null);
    if (!gatewayLabel.trim()) {
      setError(t("Profile name is required"));
      return;
    }
    if (!gatewayKey.trim()) {
      setError(t("Gateway key is required"));
      return;
    }

    setSaving(true);
    try {
      const draft = gatewayProfileDraft(gatewayKey, gatewayBaseUrl, gatewayLabel);
      await onSave(
        initial
          ? { type: "update", profile: { id: initial.id, ...draft } }
          : { type: "create", draft },
      );
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setSaving(false);
    }
  }

  const dialogTitle = editing
    ? t("Edit profile · {{label}}", { label: initial!.label })
    : t("Configure VibeWbz Gateway");

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <DialogContent className="!flex max-h-[calc(100vh-64px)] w-[min(960px,calc(100vw-32px))] max-w-[calc(100vw-32px)] flex-col overflow-hidden p-0 sm:max-w-[min(960px,calc(100vw-32px))]">
        <DialogHeader className="shrink-0 border-b border-border px-6 py-4 pr-12">
          <DialogTitle>{dialogTitle}</DialogTitle>
          <DialogDescription className="sr-only">
            {t("Configure the default gateway profile.")}
          </DialogDescription>
        </DialogHeader>

        <div className="min-h-0 flex-1 overflow-y-auto px-6 py-4 [scrollbar-gutter:stable]">
          <GatewayKeyForm
            label={gatewayLabel}
            baseUrl={gatewayBaseUrl}
            apiKey={gatewayKey}
            reveal={gatewayKeyVisible}
            onLabel={setGatewayLabel}
            onBaseUrl={setGatewayBaseUrl}
            onApiKey={setGatewayKey}
            onReveal={setGatewayKeyVisible}
          />
        </div>

        {error && (
          <div className="shrink-0 border-t border-destructive/20 bg-destructive/10 px-6 py-2 text-xs text-destructive">
            {error}
          </div>
        )}

        <DialogFooter className="shrink-0 border-t border-border px-6 py-4 sm:justify-between">
          <div />
          <div className="flex items-center gap-2">
            <Button type="button" variant="ghost" size="sm" onClick={onClose}>
              {t("Cancel")}
            </Button>
            <Button
              type="button"
              size="sm"
              onClick={handleSaveGateway}
              disabled={saving}
            >
              {saving
                ? t("Saving…")
                : editing
                  ? t("Save changes")
                  : t("Save gateway")}
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function GatewayKeyForm({
  label,
  baseUrl,
  apiKey,
  reveal,
  onLabel,
  onBaseUrl,
  onApiKey,
  onReveal,
}: {
  label: string;
  baseUrl: string;
  apiKey: string;
  reveal: boolean;
  onLabel: (value: string) => void;
  onBaseUrl: (value: string) => void;
  onApiKey: (value: string) => void;
  onReveal: (value: boolean) => void;
}) {
  const { t } = useI18n();

  return (
    <div className="mx-auto flex min-h-[260px] max-w-xl flex-col justify-center gap-4">
      <div className="flex items-start gap-3">
        <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-md border border-primary/20 bg-primary/10 text-primary">
          <KeyRound className="h-4 w-4" />
        </span>
        <div className="min-w-0">
          <div className="text-sm font-semibold">{t("Default gateway")}</div>
          <div className="mt-1 text-xs text-muted-foreground">
            {t("You can keep the default address or change it.")}
          </div>
        </div>
      </div>

      <label className="space-y-1.5">
        <span className="text-xs font-medium">{t("Profile name")}</span>
        <Input
          value={label}
          onChange={(event) => onLabel(event.target.value)}
          placeholder={GATEWAY_PROFILE_LABEL}
          className="h-9 text-[13px]"
          autoFocus
        />
      </label>

      <label className="space-y-1.5">
        <span className="text-xs font-medium">{t("Gateway address")}</span>
        <Input
          type="url"
          value={baseUrl}
          onChange={(event) => onBaseUrl(event.target.value)}
          placeholder={GATEWAY_BASE_URL}
          className="h-9 font-mono text-[13px]"
        />
      </label>

      <label className="space-y-1.5">
        <span className="text-xs font-medium">{t("Gateway key")}</span>
        <div className="relative">
          <Input
            type={reveal ? "text" : "password"}
            value={apiKey}
            onChange={(event) => onApiKey(event.target.value)}
            placeholder="sk-..."
            className="h-9 pr-9 font-mono text-[13px]"
          />
          <Button
            type="button"
            variant="ghost"
            size="icon-xs"
            className="absolute right-1 top-1 h-7 w-7"
            onClick={() => onReveal(!reveal)}
            title={reveal ? t("Hide") : t("Reveal")}
          >
            {reveal ? <EyeOff className="h-3.5 w-3.5" /> : <Eye className="h-3.5 w-3.5" />}
          </Button>
        </div>
      </label>

      <div className="rounded-md border border-border bg-muted/30 px-3 py-3 text-xs leading-5 text-muted-foreground">
        <div className="font-medium text-foreground">{t("How to get a gateway key")}</div>
        <p className="mt-1">
          {t("Open the official site to register, then top up after registration. Refresh the page after top-up and it can be used normally. If you have questions, ask in the group.")}
        </p>
        <p className="mt-1">
          {t("Open the token page, click add token, choose the features you need, then create the token.")}
        </p>
        <div className="mt-2 flex flex-wrap gap-2">
          <GatewayHelpLink href={GATEWAY_HOME_URL} label={t("Official site")} />
          <GatewayHelpLink href={GATEWAY_TOPUP_URL} label={t("Top up")} />
          <GatewayHelpLink href={GATEWAY_TOKEN_URL} label={t("Create token")} />
        </div>
      </div>
    </div>
  );
}

function GatewayHelpLink({ href, label }: { href: string; label: string }) {
  return (
    <button
      type="button"
      className="inline-flex items-center gap-1 rounded border border-border bg-background px-2 py-1 font-mono text-[11px] text-foreground hover:bg-muted"
      onClick={() => void openExternalUrl(href)}
      title={href}
    >
      <ExternalLink className="h-3 w-3" />
      {label}
    </button>
  );
}
