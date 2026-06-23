import type { DwSkill, RequirementCard, WorkflowStage, WorkflowStageState } from "./types";

export const workflowStages: WorkflowStage[] = [
  {
    id: "brainstorm",
    label: "Brainstorm",
    command: "/dw-brainstorm",
    state: "ready",
    description: "Explore product direction, research, and refactor options before planning.",
  },
  {
    id: "plan",
    label: "Plan",
    command: "/dw-plan",
    state: "ready",
    description: "Produce PRD, TechSpec, and approved task breakdown.",
  },
  {
    id: "run",
    label: "Run",
    command: "/dw-run",
    state: "pending",
    description: "Execute approved tasks with agent orchestration and atomic commits.",
  },
  {
    id: "qa",
    label: "QA",
    command: "/dw-qa",
    state: "blocked",
    description: "Validate UI/API/AI behavior with evidence-backed QA artifacts.",
  },
  {
    id: "review",
    label: "Review",
    command: "/dw-review",
    state: "blocked",
    description: "Run PRD coverage and code quality review before commit/PR.",
  },
  {
    id: "commit",
    label: "Commit",
    command: "/dw-commit",
    state: "blocked",
    description: "Create Conventional Commits after fresh verification evidence.",
  },
  {
    id: "pr",
    label: "PR",
    command: "/dw-generate-pr",
    state: "blocked",
    description: "Generate PR after verify and security gates pass.",
  },
];

export function stageCounts(stages = workflowStages) {
  return stages.reduce(
    (acc, stage) => {
      acc[stage.state] += 1;
      return acc;
    },
    { ready: 0, active: 0, complete: 0, pending: 0, blocked: 0 },
  );
}

export type DwPlanMode = "default" | "prd" | "techspec" | "tasks";

export function composeDwPlanCommand(
  mode: DwPlanMode,
  value: string,
  selectedSkills: Pick<DwSkill, "name">[] = [],
) {
  const trimmedValue = value.trim();
  const command =
    mode === "default"
      ? ["/dw-plan", quoteCommandArg(trimmedValue)].filter(Boolean).join(" ")
      : ["/dw-plan", mode, quoteCommandArg(trimmedValue)].filter(Boolean).join(" ");
  const skillContext = selectedSkills
    .map((skill) => skill.name.trim())
    .filter(Boolean)
    .map((name) => `$${name}`)
    .join(" ");

  return [command, skillContext].filter(Boolean).join(" ");
}

export function quoteCommandArg(value: string) {
  if (!value) return "";
  return `"${value.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;
}

export function requirementPrdSlug(card: Pick<RequirementCard, "prd_slug" | "slug">) {
  const explicit = card.prd_slug?.trim();
  return explicit || `prd-${card.slug}`;
}

export function requirementPipelineCommands(
  card: Pick<RequirementCard, "title" | "slug" | "prd_slug">,
) {
  const prdSlug = requirementPrdSlug(card);
  return [
    { label: "Brainstorm", command: `/dw-brainstorm ${quoteCommandArg(card.title)}` },
    { label: "Plan", command: `/dw-plan ${quoteCommandArg(card.title)}` },
    { label: "Goal", command: `/dw-goal ${quoteCommandArg(prdSlug)}` },
    { label: "Run", command: `/dw-run ${quoteCommandArg(prdSlug)}` },
    { label: "Review", command: `/dw-review ${quoteCommandArg(prdSlug)}` },
    { label: "QA", command: `/dw-qa ${quoteCommandArg(prdSlug)}` },
    { label: "Commit", command: "/dw-commit" },
    { label: "Local PR", command: "/dw-generate-pr" },
  ];
}

// Stage ids are now defined by the workbench schema (see workbenchSchema.ts),
// so this is just an alias for any schema-defined phase id.
export type RequirementStageId = string;

export function isArchivedRequirement(card: Pick<RequirementCard, "status">) {
  return card.status === "archived";
}

export function workflowStateLabel(state: WorkflowStageState | string) {
  switch (state) {
    case "ready":
      return "Ready";
    case "active":
      return "Active";
    case "complete":
      return "Complete";
    case "pending":
      return "Pending";
    case "blocked":
      return "Blocked";
    default:
      return state;
  }
}

export function normalizeWorkflowStageState(state: string): WorkflowStageState {
  if (
    state === "ready" ||
    state === "active" ||
    state === "complete" ||
    state === "pending" ||
    state === "blocked"
  ) {
    return state;
  }
  return "pending";
}
