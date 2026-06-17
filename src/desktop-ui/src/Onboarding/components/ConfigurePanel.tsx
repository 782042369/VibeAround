import { KeyRound } from "lucide-react";
import { useI18n } from "@va/i18n";

export function ConfigurePanel({
  finishError,
}: {
  finishError: string | null;
}) {
  const { t } = useI18n();

  return (
    <div className="mx-auto flex min-h-full w-full max-w-4xl items-center py-4">
      <div className="w-full space-y-8">
        <div className="px-4 py-10 text-center">
          <CheckReady />
          <div className="mt-3 text-sm font-medium">{t("Ready to launch")}</div>
          <p className="mt-1 text-xs text-muted-foreground">
            {t("Add your gateway key on the Launch screen, then open Claude or Codex.")}
          </p>
        </div>

        {finishError && (
          <div className="rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-xs text-destructive">
            {finishError}
          </div>
        )}
      </div>
    </div>
  );
}

function CheckReady() {
  return (
    <div className="mx-auto flex h-10 w-10 items-center justify-center rounded-md bg-primary/10 text-primary">
      <KeyRound className="h-5 w-5" />
    </div>
  );
}
