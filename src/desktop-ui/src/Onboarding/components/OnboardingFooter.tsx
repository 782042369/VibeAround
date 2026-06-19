import type { ReactNode } from "react";
import { ArrowLeft } from "lucide-react";
import { useI18n } from "@va/i18n";

import { Button } from "@/components/ui/button";

import type { WizardStepId } from "../wizardTypes";

export interface PrimaryAction {
  label: string;
  icon: ReactNode;
  disabled: boolean;
  run: () => void;
}

export type FooterAction = PrimaryAction;

export function OnboardingFooter({
  activeStep,
  activeIndex,
  running,
  finishing,
  primaryAction,
  secondaryAction,
  onBack,
  onCancel,
}: {
  activeStep: WizardStepId;
  activeIndex: number;
  running: boolean;
  finishing: boolean;
  primaryAction: PrimaryAction;
  secondaryAction?: FooterAction | null;
  onBack: () => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();

  return (
    <footer className="relative flex min-h-16 items-center gap-3 border-t border-border px-5 py-2">
      <div className="flex items-center gap-2">
        <Button
          type="button"
          variant="outline"
          onClick={onBack}
          disabled={activeIndex === 0 || running || finishing}
        >
          <ArrowLeft className="h-4 w-4" />
          {t("Back")}
        </Button>
      </div>
      <div className="pointer-events-none absolute left-1/2 max-w-md -translate-x-1/2 px-4 text-center text-xs leading-5 text-muted-foreground">
        <div>
          {t("Keep the defaults if you are not sure; everything can be changed later.")}
        </div>
        <div>
          {t("Only environment installation is handled here; Claude and Codex configs stay untouched.")}
        </div>
      </div>
      <div className="ml-auto flex items-center gap-2">
        {running && activeStep === "install" && (
          <Button type="button" variant="outline" onClick={onCancel}>
            {t("Cancel")}
          </Button>
        )}
        {secondaryAction && (
          <Button
            type="button"
            variant="outline"
            onClick={secondaryAction.run}
            disabled={secondaryAction.disabled}
          >
            {secondaryAction.icon}
            {secondaryAction.label}
          </Button>
        )}
        <Button
          type="button"
          onClick={primaryAction.run}
          disabled={primaryAction.disabled}
        >
          {primaryAction.icon}
          {primaryAction.label}
        </Button>
      </div>
    </footer>
  );
}
