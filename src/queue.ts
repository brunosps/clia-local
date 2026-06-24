import type { ChecklistItem, Project, RequirementCard } from "./types";

export type QueueBucket = "pending" | "doing" | "validating" | "done";

/** The four fixed kanban columns, in order, with their pt-BR labels. */
export const QUEUE_BUCKETS: Array<{ id: QueueBucket; label: string }> = [
  { id: "pending", label: "A fazer" },
  { id: "doing", label: "Fazendo" },
  { id: "validating", label: "Validando" },
  { id: "done", label: "Feito" },
];

/** Canonical status persisted to the backend when a card lands in each column. */
const bucketStatusValue: Record<QueueBucket, string> = {
  pending: "todo",
  doing: "doing",
  validating: "validating",
  done: "done",
};

export function bucketCanonicalStatus(bucket: QueueBucket): string {
  return bucketStatusValue[bucket];
}

export type QueueCardPriority = "high" | "medium" | "low";

export type QueueCard = {
  id: string;
  cardId: number;
  publicId: string;
  title: string;
  body: string;
  status: string;
  bucket: QueueBucket;
  priority: QueueCardPriority;
  updatedAt: string | null;
  workspaceId: number;
  projectIds: number[];
  projectNames: string[];
  checklistTotal: number;
  checklistDone: number;
  agentPrompt: string;
  raw: RequirementCard;
};

export type QueueModel = {
  items: QueueCard[];
  buckets: Record<QueueBucket, QueueCard[]>;
};

const priorityRank: Record<QueueCardPriority, number> = { high: 3, medium: 2, low: 1 };

function normalize(value: string): string {
  return value.toLowerCase().replace(/[\s_-]+/g, "");
}

// Tolerant mapping so legacy statuses (draft/running/reviewing/…) still land in
// a sensible column alongside the canonical todo/doing/validating/done.
const bucketByStatus = new Map<string, QueueBucket>([
  ["todo", "pending"],
  ["draft", "pending"],
  ["backlog", "pending"],
  ["brainstorm", "pending"],
  ["brainstorming", "pending"],
  ["plan", "pending"],
  ["planned", "pending"],
  ["pending", "pending"],
  ["doing", "doing"],
  ["run", "doing"],
  ["running", "doing"],
  ["inprogress", "doing"],
  ["validating", "validating"],
  ["review", "validating"],
  ["reviewing", "validating"],
  ["qa", "validating"],
  ["security", "validating"],
  ["done", "done"],
  ["commit", "done"],
  ["localpr", "done"],
  ["readyforpr", "done"],
  ["complete", "done"],
  ["completed", "done"],
]);

export function statusBucket(status: string | null | undefined): QueueBucket {
  if (!status) return "pending";
  return bucketByStatus.get(normalize(status)) ?? "pending";
}

/** Parse a card's `checklist_json` into a normalized list of subtasks. */
export function parseChecklist(json: string | null | undefined): ChecklistItem[] {
  if (!json) return [];
  try {
    const parsed = JSON.parse(json);
    if (!Array.isArray(parsed)) return [];
    return parsed
      .filter(
        (item): item is Record<string, unknown> => Boolean(item) && typeof item === "object",
      )
      .map((item, index) => ({
        id: typeof item.id === "string" && item.id ? item.id : `item-${index}`,
        text: typeof item.text === "string" ? item.text : "",
        done: item.done === true,
      }));
  } catch {
    return [];
  }
}

export function serializeChecklist(items: ChecklistItem[]): string {
  return JSON.stringify(items);
}

function queuePriority(value: string | null | undefined): QueueCardPriority {
  const normalized = normalize(value ?? "medium");
  if (normalized === "high" || normalized === "urgent") return "high";
  if (normalized === "low") return "low";
  return "medium";
}

function cardProjectIds(card: RequirementCard): number[] {
  if (card.project_ids.length) return card.project_ids;
  return card.project_id != null ? [card.project_id] : [];
}

/**
 * Build the kanban model for a single workspace's tasks. Archived cards are
 * excluded; an optional `projectId` filters to tasks linked to that project.
 */
export function buildQueue(
  cards: RequirementCard[],
  projects: Project[] = [],
  options: { projectId?: number | null } = {},
): QueueModel {
  const projectNameById = new Map(projects.map((project) => [project.id, project.name]));
  const projectFilter = options.projectId ?? null;
  const items = cards
    .filter((card) => card.status !== "archived" && !card.archived_at)
    .filter((card) => {
      if (projectFilter == null) return true;
      return cardProjectIds(card).includes(projectFilter);
    })
    .map((card) => {
      const checklist = parseChecklist(card.checklist_json);
      const projectIds = cardProjectIds(card);
      return {
        id: String(card.id),
        cardId: card.id,
        publicId: card.public_id || String(card.id),
        title: card.title?.trim() || "Sem título",
        body: card.body ?? "",
        status: card.status,
        bucket: statusBucket(card.status),
        priority: queuePriority(card.priority),
        updatedAt: card.updated_at ?? null,
        workspaceId: card.workspace_id,
        projectIds,
        projectNames: projectIds
          .map((id) => projectNameById.get(id))
          .filter((name): name is string => Boolean(name)),
        checklistTotal: checklist.length,
        checklistDone: checklist.filter((item) => item.done).length,
        agentPrompt: card.agent_prompt ?? "",
        raw: card,
      } satisfies QueueCard;
    })
    .sort((a, b) => {
      const diff = priorityRank[b.priority] - priorityRank[a.priority];
      if (diff !== 0) return diff;
      const bDate = b.updatedAt ? Date.parse(b.updatedAt) : 0;
      const aDate = a.updatedAt ? Date.parse(a.updatedAt) : 0;
      return bDate - aDate;
    });
  return {
    items,
    buckets: {
      pending: items.filter((item) => item.bucket === "pending"),
      doing: items.filter((item) => item.bucket === "doing"),
      validating: items.filter((item) => item.bucket === "validating"),
      done: items.filter((item) => item.bucket === "done"),
    },
  };
}
