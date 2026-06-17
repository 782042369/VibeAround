export type WizardStepId = "agents" | "install" | "configure";

export interface WizardStep {
  id: WizardStepId;
  label: string;
}

export const WIZARD_STEPS: WizardStep[] = [
  { id: "agents", label: "Agents" },
  { id: "install", label: "Install" },
  { id: "configure", label: "Config" },
];
