import {
  CheckCircle2,
  ExternalLink,
  KeyRound,
  MessageCircle,
  RefreshCw,
  Wrench,
} from "lucide-react";
import type { ReactNode } from "react";
import { useI18n } from "@va/i18n";

import { LanguageMenu } from "./components/LanguageMenu";
import { Button } from "./components/ui/button";
import { openExternalUrl } from "./lib/api";
import { AI_MODEL_GUIDE_URL, GATEWAY_TOKEN_URL } from "./lib/guides";
import { cn } from "./lib/utils";
import wechatQrUrl from "../../../assets/community/ScreenShot_2026-06-19_171941_284.png";

export function SetupGuide({
  onConfigureEnvironment,
}: {
  onConfigureEnvironment: () => void;
}) {
  const { t } = useI18n();
  const isMacTitlebar =
    typeof navigator !== "undefined" && /Mac/.test(navigator.platform);

  return (
    <div className="flex h-full flex-col bg-background">
      <header
        className={cn(
          "relative flex h-12 items-center justify-between border-b border-border pr-3",
          isMacTitlebar ? "pl-[82px]" : "pl-3",
        )}
      >
        <div
          data-tauri-drag-region
          aria-hidden="true"
          className="absolute inset-0 z-0"
        />
        <div className="relative z-10 flex min-w-0 items-center gap-1.5 whitespace-nowrap">
          <img
            src="/brand/vibewbz-mark.svg"
            alt=""
            className="h-5 w-5 shrink-0"
            aria-hidden="true"
            draggable={false}
          />
          <span className="text-[13px] font-semibold text-foreground">
            VibeWbz
          </span>
          <span className="font-mono text-[10px] text-muted-foreground/60">
            @{__APP_VERSION_LABEL__}
          </span>
        </div>
        <div className="relative z-10 flex items-center gap-2">
          <LanguageMenu />
        </div>
      </header>

      <main className="min-h-0 flex-1 overflow-y-auto">
        <div className="mx-auto grid min-h-full w-full max-w-6xl gap-8 px-6 py-8 lg:grid-cols-[minmax(0,1fr)_300px]">
          <section className="flex min-w-0 flex-col justify-center">
            <div className="max-w-3xl space-y-6">
              <div className="inline-flex items-center gap-2 rounded-md border border-emerald-500/25 bg-emerald-500/10 px-3 py-1.5 text-xs font-medium text-emerald-700 dark:text-emerald-300">
                <CheckCircle2 className="h-3.5 w-3.5" />
                {t("Base environment ready")}
              </div>

              <div className="space-y-3">
                <h1 className="text-3xl font-semibold leading-tight sm:text-4xl">
                  {t("Next, configure your AI model and create a gateway token.")}
                </h1>
                <p className="max-w-2xl text-sm leading-6 text-muted-foreground">
                  {t(
                    "VibeWbz now only prepares the local coding environment. Follow the AI model guide, then create a gateway token.",
                  )}
                </p>
              </div>

              <div className="grid gap-3 md:grid-cols-3">
                <GuideStep
                  icon={<Wrench className="h-4 w-4" />}
                  title={t("Configure environment")}
                  body={t("Re-run the basic environment check when you change computers or tools.")}
                  action={
                    <Button type="button" variant="outline" size="sm" onClick={onConfigureEnvironment}>
                      <RefreshCw className="h-4 w-4" />
                      {t("Check environment")}
                    </Button>
                  }
                />
                <GuideStep
                  icon={<ExternalLink className="h-4 w-4" />}
                  title={t("Configure AI model")}
                  body={t("Open the guide and follow it to configure the AI model for Claude Code or Codex.")}
                  action={
                    <Button type="button" size="sm" onClick={() => void openExternalUrl(AI_MODEL_GUIDE_URL)}>
                      <ExternalLink className="h-4 w-4" />
                      {t("Configuration guide")}
                    </Button>
                  }
                />
                <GuideStep
                  icon={<KeyRound className="h-4 w-4" />}
                  title={t("Create gateway token")}
                  body={t("Go to the gateway, create a token, then paste that token into CCS.")}
                  action={
                    <Button type="button" variant="outline" size="sm" onClick={() => void openExternalUrl(GATEWAY_TOKEN_URL)}>
                      <KeyRound className="h-4 w-4" />
                      {t("Create token")}
                    </Button>
                  }
                />
              </div>
            </div>
          </section>

          <aside className="flex min-w-0 flex-col justify-center">
            <section className="overflow-hidden rounded-md border border-border bg-card">
              <div className="border-b border-border px-4 py-3">
                <div className="flex items-center gap-2 text-sm font-semibold">
                  <MessageCircle className="h-4 w-4 text-primary" />
                  {t("Join the group for $5 trial credit")}
                </div>
                <p className="mt-1 text-xs leading-5 text-muted-foreground">
                  {t("Scan the QR code to join the community group and claim the trial credit.")}
                </p>
              </div>
              <div className="bg-background p-4">
                <img
                  src={wechatQrUrl}
                  alt={t("Community group QR code")}
                  className="mx-auto w-full max-w-[220px] rounded-md border border-border object-contain"
                />
              </div>
            </section>
          </aside>
        </div>
      </main>

      <footer className="shrink-0 border-t border-border px-3 py-1.5 text-center text-[11px] leading-4 text-muted-foreground">
        {t("仅供社区学习交流使用，所有安装源均来自官方来源。")}
      </footer>
    </div>
  );
}

function GuideStep({
  icon,
  title,
  body,
  action,
}: {
  icon: ReactNode;
  title: string;
  body: string;
  action: ReactNode;
}) {
  return (
    <section className="flex min-h-[190px] flex-col rounded-md border border-border bg-card p-4">
      <div className="flex h-8 w-8 items-center justify-center rounded-md bg-primary/10 text-primary">
        {icon}
      </div>
      <div className="mt-4 text-sm font-semibold">{title}</div>
      <p className="mt-2 flex-1 text-xs leading-5 text-muted-foreground">
        {body}
      </p>
      <div className="mt-4">{action}</div>
    </section>
  );
}
