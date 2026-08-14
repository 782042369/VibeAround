import { invoke } from "@tauri-apps/api/core";
import {
  ArrowRight,
  BookOpen,
  Check,
  Download,
  ExternalLink,
  Gift,
  KeyRound,
  Loader2,
} from "lucide-react";
import { useCallback, useEffect, useState, type ReactNode } from "react";
import { useI18n } from "@va/i18n";

import { LanguageMenu } from "./components/LanguageMenu";
import { Button } from "./components/ui/button";
import { openExternalUrl } from "./lib/api";
import { AI_MODEL_GUIDE_URL, GATEWAY_TOKEN_URL } from "./lib/guides";
import { cn } from "./lib/utils";
import type { AgentSummary } from "./Onboarding/types";
import relayLogoUrl from "../../../Logo.png";

const CODEX_DESKTOP_ID = "codex-desktop";
const WINDOWS_CODEX_DOWNLOAD_URL =
  "https://codexapp.agentsmirror.com/manager/latest/CodexAppManager_x64-setup.exe";

export function SetupGuide() {
  const { t } = useI18n();
  const isMacTitlebar =
    typeof navigator !== "undefined" && /Mac/.test(navigator.platform);
  const [codexDownloadUrl, setCodexDownloadUrl] = useState<string | null>(
    typeof navigator !== "undefined" && /Windows/.test(navigator.userAgent)
      ? WINDOWS_CODEX_DOWNLOAD_URL
      : null,
  );
  const [openingDownload, setOpeningDownload] = useState(false);

  useEffect(() => {
    void invoke<AgentSummary[]>("list_agents")
      .then((agents) => {
        const codexDesktop = agents.find(
          (agent) => agent.id === CODEX_DESKTOP_ID,
        );
        if (codexDesktop?.download_url) {
          setCodexDownloadUrl(codexDesktop.download_url);
        }
      })
      .catch((error) => {
        console.warn("failed to load Codex Desktop download URL", error);
      });
  }, []);

  const downloadCodexDesktop = useCallback(async () => {
    if (!codexDownloadUrl || openingDownload) return;
    setOpeningDownload(true);
    try {
      await openExternalUrl(codexDownloadUrl);
    } finally {
      setOpeningDownload(false);
    }
  }, [codexDownloadUrl, openingDownload]);

  return (
    <div className="dark flex h-full flex-col bg-[#0f1718] text-[#f2f6f5]">
      <header
        className={cn(
          "relative flex h-12 shrink-0 items-center justify-between border-b border-[#263638] bg-[#121a1b] pr-3",
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
          <span className="text-[13px] font-semibold text-white">VibeWbz</span>
          <span className="font-mono text-[10px] text-[#6e8183]">
            @{__APP_VERSION_LABEL__}
          </span>
        </div>
        <div className="relative z-10 flex items-center gap-1">
          <LanguageMenu />
        </div>
      </header>

      <main className="min-h-0 flex-1 overflow-y-auto">
        <div className="mx-auto min-h-full w-full max-w-[1180px] px-5 py-5 sm:px-7 lg:px-10 lg:py-7">
          <section>
            <div className="min-w-0 max-w-[980px]">
              <div className="inline-flex items-center gap-2 text-[11px] font-semibold text-[#52d4b8]">
                <span className="h-1.5 w-1.5 rounded-full bg-[#41c7a4]" />
                {t("Get Codex ready for this computer")}
              </div>

              <h1 className="mt-3 text-[32px] font-bold leading-[1.12] tracking-normal text-white sm:text-[38px]">
                {t("One-click install")} <span className="text-[#48cdb0]">Codex Desktop</span>
                <br />
                {t("Start AI coding in three steps")}
              </h1>
              <p className="mt-2.5 max-w-2xl text-xs leading-5 text-[#93a5a7]">
                {t(
                  "Automatically match the official version for your system without changing existing Codex settings.",
                )}
              </p>

              <div className="mt-5 grid max-w-[980px] items-center gap-3 rounded-md border border-[#554a1d] border-l-[3px] border-l-[#f4cd35] bg-[#191b12] px-3.5 py-3 sm:grid-cols-[max-content_minmax(0,1fr)] sm:gap-4">
                <div className="flex items-center gap-2.5">
                  <img
                    src={relayLogoUrl}
                    alt=""
                    aria-hidden="true"
                    className="h-10 w-10 shrink-0 rounded-full object-cover"
                    draggable={false}
                  />
                  <span className="max-w-[205px] text-sm font-bold leading-5 text-white">
                    {t("天才第一步-歪歪纸尿裤")}
                  </span>
                </div>
                <div className="border-t border-[#635a2e] pt-3 sm:border-l sm:border-t-0 sm:py-0 sm:pl-4">
                  <div className="text-sm font-bold leading-5 text-white">
                    {t("We don't produce Tokens. We just move Tokens.")}
                  </div>
                  <div className="mt-1 text-[11px] font-medium text-[#f0d66a]">
                    {t("Native models · Transparent usage · No dilution")}
                  </div>
                </div>
                <div className="flex flex-wrap items-center justify-between gap-3 border-t border-[#635a2e] pt-3 sm:col-span-2">
                  <div className="flex min-w-0 items-center gap-2.5">
                    <span className="grid h-9 w-9 shrink-0 place-items-center rounded-md bg-[#f4cd35] text-[#211d0e]">
                      <Gift className="h-5 w-5" />
                    </span>
                    <div className="min-w-0">
                      <div className="flex flex-wrap items-baseline gap-x-2 gap-y-0.5">
                        <span className="text-[11px] font-bold text-[#ffe27b]">
                          {t("First use bonus")}
                        </span>
                        <strong className="text-[24px] leading-none text-[#ffe27b]">
                          $5
                        </strong>
                        <span className="text-[10px] text-[#c8b96c]">
                          {t("relay credit")}
                        </span>
                      </div>
                      <div className="mt-1 text-[10px] text-[#b8aa6b]">
                        {t("Use it on the relay")}
                      </div>
                    </div>
                  </div>
                  <Button
                    type="button"
                    onClick={() => void openExternalUrl(GATEWAY_TOKEN_URL)}
                    className="h-9 shrink-0 bg-[#f4cd35] px-3.5 text-[11px] font-bold text-[#211d0e] hover:bg-[#ffe16a]"
                  >
                    {t("Claim now")}
                    <ArrowRight className="h-4 w-4" />
                  </Button>
                </div>
              </div>
            </div>
          </section>

          <section className="mt-5">
            <div className="mb-3 flex flex-wrap items-baseline gap-x-2 gap-y-1">
              <h2 className="text-sm font-bold text-white">
                {t("From installation to first use")}
              </h2>
              <span className="text-[10px] text-[#6f8284]">
                {t("Complete in order in just a few minutes")}
              </span>
            </div>

            <div className="grid gap-3 md:grid-cols-3">
              <FlowStep
                number="01"
                label={t("Install Codex Desktop")}
                status={t("Current step")}
                active
                icon={<Download className="h-4 w-4" />}
                title={t("One-click download")}
                body={t(
                  "Automatically match and download the official Codex Desktop build for this computer.",
                )}
                facts={[
                  t("Correct version selected automatically"),
                  t("Official source"),
                  t("Existing settings stay untouched"),
                ]}
                action={
                  <Button
                    type="button"
                    onClick={() => void downloadCodexDesktop()}
                    disabled={!codexDownloadUrl || openingDownload}
                    className="h-9 w-full bg-[#0b9484] text-[11px] font-bold text-white hover:bg-[#10a895]"
                  >
                    {openingDownload ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      <Download className="h-4 w-4" />
                    )}
                    {openingDownload
                      ? t("Opening...")
                      : t("Download and install Codex Desktop")}
                  </Button>
                }
                meta={t("Download the installer, then follow the prompts")}
              />

              <FlowStep
                number="02"
                label={t("View configuration guide")}
                status={t("After installation")}
                icon={<BookOpen className="h-4 w-4" />}
                title={t("Understand setup first")}
                body={t(
                  "See where to create and enter your Token before configuring Codex Desktop.",
                )}
                facts={[
                  t("Clear step-by-step instructions"),
                  t("Know what to prepare"),
                  t("Community support available"),
                ]}
                action={
                  <Button
                    type="button"
                    variant="outline"
                    onClick={() => void openExternalUrl(AI_MODEL_GUIDE_URL)}
                    className="h-9 w-full border-[#344547] bg-[#1a2324] text-[11px] font-bold text-[#b3c3c4] hover:bg-[#222e2f] hover:text-white"
                  >
                    <BookOpen className="h-4 w-4" />
                    {t("Open Codex configuration guide")}
                    <ExternalLink className="h-3.5 w-3.5" />
                  </Button>
                }
                meta={t("Continue after reading the guide")}
              />

              <FlowStep
                number="03"
                label={t("Get a Token and use it")}
                status={t("Final step")}
                icon={<KeyRound className="h-4 w-4" />}
                title={t("Go to YY Relay")}
                body={t(
                  "Create a Token, claim your first-use $5 credit, enter it in Codex Desktop as shown in the guide, and start coding.",
                )}
                facts={[
                  t("Native model capabilities"),
                  t("Transparent usage records"),
                  t("New-user trial credit"),
                ]}
                action={
                  <Button
                    type="button"
                    variant="outline"
                    onClick={() => void openExternalUrl(GATEWAY_TOKEN_URL)}
                    className="h-9 w-full border-[#d0ad32] bg-[#2c2918] text-[11px] font-bold text-[#f0cf58] hover:bg-[#3a351d] hover:text-[#ffe36b]"
                  >
                    <KeyRound className="h-4 w-4" />
                    {t("Go to YY Relay")}
                    <ArrowRight className="h-3.5 w-3.5" />
                  </Button>
                }
                meta={t("Create a Token and finish setup using the guide")}
              />
            </div>
          </section>
        </div>
      </main>

      <footer className="flex shrink-0 items-center justify-between gap-4 border-t border-[#253234] px-4 py-1.5 text-[9px] leading-4 text-[#607476]">
        <span>
          {t(
            "VibeWbz provides download assistance; installers come from the official distribution channel.",
          )}
        </span>
        <span className="hidden text-right sm:block">
          {t("天才第一步-歪歪纸尿裤 · 原生模型，用量透明")}
        </span>
      </footer>
    </div>
  );
}

function FlowStep({
  number,
  label,
  status,
  active = false,
  icon,
  title,
  body,
  facts,
  action,
  meta,
}: {
  number: string;
  label: string;
  status: string;
  active?: boolean;
  icon: ReactNode;
  title: string;
  body: string;
  facts: string[];
  action: ReactNode;
  meta: string;
}) {
  return (
    <article
      className={cn(
        "flex min-h-[260px] min-w-0 flex-col overflow-hidden rounded-md border bg-[#141c1d]",
        active ? "border-[#2d7669]" : "border-[#283537]",
      )}
    >
      <div
        className={cn(
          "flex h-[52px] shrink-0 items-center gap-2.5 border-b px-3.5",
          active
            ? "border-[#2d7669] bg-[#172724]"
            : "border-[#283537] bg-[#172021]",
        )}
      >
        <span
          className={cn(
            "grid h-6 w-6 shrink-0 place-items-center rounded-md font-mono text-[10px]",
            active
              ? "bg-[#0e8f7f] text-white"
              : "bg-[#263537] text-[#a9b8ba]",
          )}
        >
          {number}
        </span>
        <span className="min-w-0 flex-1 truncate text-[11px] font-bold text-white">
          {label}
        </span>
        <span
          className={cn(
            "shrink-0 text-[9px]",
            active ? "text-[#4dd0ae]" : "text-[#6d8183]",
          )}
        >
          {status}
        </span>
      </div>

      <div className="flex flex-1 flex-col p-4">
        <div className="flex items-center gap-2 text-sm font-bold text-white">
          <span className={active ? "text-[#45c9ad]" : "text-[#91a4a6]"}>
            {icon}
          </span>
          {title}
        </div>
        <p className="mt-2 min-h-10 text-[10px] leading-[1.55] text-[#849597]">
          {body}
        </p>
        <div className="my-3 grid gap-1.5">
          {facts.map((fact) => (
            <div
              key={fact}
              className="flex items-center gap-2 text-[9px] text-[#a9b7b8]"
            >
              <Check className="h-3 w-3 shrink-0 text-[#44cbaa]" />
              <span>{fact}</span>
            </div>
          ))}
        </div>
        <div className="mt-auto">{action}</div>
        <div className="mt-2 text-center text-[8px] text-[#627577]">
          {meta}
        </div>
      </div>
    </article>
  );
}
