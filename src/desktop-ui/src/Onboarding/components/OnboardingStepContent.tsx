import { AgentDecisionPanel } from "./AgentDecisionPanel";
import { ConfigurePanel } from "./ConfigurePanel";
import { InstallPanel } from "./InstallPanel";
import type { AgentId } from "../constants";
import type {
  AgentSummary,
  StartkitChoices,
  StartkitItemReport,
} from "../types";
import type { WizardStepId } from "../wizardTypes";

export function OnboardingStepContent({
  activeStep,
  agents,
  enabledAgents,
  reportsById,
  scanning,
  onToggleAgent,
  groupedReports,
  reports,
  running,
  complete,
  finalStatus,
  startkitError,
  choices,
  hasInstallChoices,
  finishError,
}: {
  activeStep: WizardStepId;
  agents: AgentSummary[];
  enabledAgents: Set<AgentId>;
  reportsById: Map<string, StartkitItemReport>;
  scanning: boolean;
  onToggleAgent: (id: AgentId) => void;
  groupedReports: Array<{ id: string; reports: StartkitItemReport[] }>;
  reports: StartkitItemReport[];
  running: boolean;
  complete: boolean;
  finalStatus: string | null;
  startkitError: string | null;
  choices: StartkitChoices;
  hasInstallChoices: boolean;
  finishError: string | null;
}) {
  return (
    <section
      key={activeStep}
      className="min-h-0 overflow-y-auto p-5 animate-in fade-in slide-in-from-bottom-1 duration-300"
    >
      {activeStep === "agents" && (
        <AgentDecisionPanel
          agents={agents}
          enabledAgents={enabledAgents}
          reports={reportsById}
          onToggleAgent={onToggleAgent}
        />
      )}

      {activeStep === "install" && (
        <InstallPanel
          groupedReports={groupedReports}
          reports={reports}
          scanning={scanning}
          running={running}
          complete={complete}
          finalStatus={finalStatus}
          error={startkitError}
          choices={choices}
          hasInstallChoices={hasInstallChoices}
        />
      )}

      {activeStep === "configure" && (
        <ConfigurePanel
          finishError={finishError}
        />
      )}
    </section>
  );
}
