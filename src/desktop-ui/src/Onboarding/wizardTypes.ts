export type WizardStepId = "agents" | "install";

export interface WizardStep {
  id: WizardStepId;
  label: string;
}

export const WIZARD_STEPS: WizardStep[] = [
  { id: "agents", label: "Agents" },
  { id: "install", label: "Install" },
];
