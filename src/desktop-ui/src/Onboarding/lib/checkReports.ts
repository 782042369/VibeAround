import type {
  AgentSummary,
  StartkitItemReport,
  StartkitStatus,
} from "../types";

export function itemCheckSignature(...parts: Array<string | number | boolean>): string {
  return parts.join("|");
}

export function agentIdFromReport(report: StartkitItemReport): string | null {
  const match = /^agents\.([^.]+)\.cli$/.exec(report.id);
  return match?.[1] ?? null;
}

export function agentCheckingReport(
  agentId: string,
  agents: AgentSummary[],
  message: string,
): StartkitItemReport {
  const agent = agents.find((item) => item.id === agentId);
  return {
    id: `agents.${agentId}.cli`,
    label: agent?.display_name ?? agentId,
    group: "agents",
    category: "agents",
    status: "running",
    message,
    actions: [],
    secret: false,
  };
}

export function mergeReportsById(
  previous: StartkitItemReport[],
  updates: StartkitItemReport[],
): StartkitItemReport[] {
  const merged = new Map(previous.map((report) => [report.id, report]));
  for (const report of updates) {
    merged.set(report.id, report);
  }
  return Array.from(merged.values());
}

export function markReportsUpdating(
  reports: StartkitItemReport[],
  reportIds: Set<string>,
  message: string,
): StartkitItemReport[] {
  return reports.map((report) =>
    reportIds.has(report.id)
      ? { ...report, status: "running" as StartkitStatus, message }
      : report,
  );
}

export function groupReportsFromReports(reports: StartkitItemReport[]) {
  const groups = new Map<string, StartkitItemReport[]>();
  for (const report of reports) {
    if (!groups.has(report.group)) groups.set(report.group, []);
    groups.get(report.group)!.push(report);
  }
  return Array.from(groups.entries()).map(([id, groupReports]) => ({
    id,
    reports: groupReports,
  }));
}
