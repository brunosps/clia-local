import type { WiredCloudCard, WiredCloudWorkspace } from "./types";
import type { WorkbenchPhase, WorkbenchSchema } from "./workbenchSchema";

export type QueueBucket = "pending" | "doing" | "validating" | "done";

export type QueueCard = {
  id: string;
  publicId: string;
  title: string;
  body: string;
  status: string;
  bucket: QueueBucket;
  bucketStatus: Record<QueueBucket, string>;
  priority: "high" | "medium" | "low" | "none";
  updatedAt: string | null;
  workspaceId: string | null;
  workspaceName: string;
  projectName: string | null;
  assigneeName: string | null;
  needsInstall: boolean;
  wiredTaskStatus: string | null;
  wiredTaskKind: string | null;
  documentIds: string[];
  raw: WiredCloudCard;
};

export type QueueModel = {
  items: QueueCard[];
  buckets: Record<QueueBucket, QueueCard[]>;
};

const bucketOrder: QueueBucket[] = ["pending", "doing", "validating", "done"];
const priorityRank = new Map([
  ["urgent", 4],
  ["high", 3],
  ["medium", 2],
  ["med", 2],
  ["normal", 2],
  ["low", 1],
  ["none", 0],
]);

const defaultBucketStageIds: Record<QueueBucket, string[]> = {
  pending: ["backlog", "brainstorm", "plan"],
  doing: ["run"],
  validating: ["review", "qa", "security"],
  done: ["commit", "localpr", "local-pr", "done"],
};

function text(value: unknown): string | null {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

function phaseId(phase: Pick<WorkbenchPhase, "id">) {
  return phase.id.trim();
}

function normalize(value: string) {
  return value.toLowerCase().replace(/[\s_-]+/g, "");
}

function stageBucket(status: string | null, phases: Pick<WorkbenchPhase, "id">[]): QueueBucket {
  const normalized = normalize(status ?? "");
  if (!normalized) return "pending";
  for (const bucket of bucketOrder) {
    if (defaultBucketStageIds[bucket].some((stage) => normalize(stage) === normalized)) {
      return bucket;
    }
  }
  const index = phases.findIndex((phase) => normalize(phaseId(phase)) === normalized);
  if (index < 0 || phases.length < 2) return "pending";
  const ratio = index / Math.max(phases.length - 1, 1);
  if (ratio >= 0.82) return "done";
  if (ratio >= 0.5) return "validating";
  if (ratio >= 0.33) return "doing";
  return "pending";
}

function firstStageForBucket(bucket: QueueBucket, phases: Pick<WorkbenchPhase, "id">[]) {
  const defaultIds = defaultBucketStageIds[bucket].map(normalize);
  const matched = phases.find((phase) => defaultIds.includes(normalize(phaseId(phase))));
  if (matched) return phaseId(matched);
  if (!phases.length) return defaultBucketStageIds[bucket][0] ?? bucket;
  const indexByBucket: Record<QueueBucket, number> = {
    pending: 0,
    doing: Math.floor(phases.length / 3),
    validating: Math.floor((phases.length * 2) / 3),
    done: phases.length - 1,
  };
  return phaseId(phases[Math.min(indexByBucket[bucket], phases.length - 1)]);
}

function priorityValue(card: WiredCloudCard) {
  const priority = normalize(text(card.priority) ?? "medium");
  return priorityRank.get(priority) ?? priorityRank.get("medium") ?? 2;
}

function queuePriority(card: WiredCloudCard): QueueCard["priority"] {
  const priority = normalize(text(card.priority) ?? "medium");
  if (priority === "urgent" || priority === "high") return "high";
  if (priority === "low") return "low";
  if (priority === "none") return "none";
  return "medium";
}

export function buildQueue(
  cards: WiredCloudCard[],
  currentUserId: string | null | undefined,
  installedWorkspaceIds: Iterable<string>,
  activeWorkflow?: Pick<WorkbenchSchema, "phases"> | null,
): QueueModel {
  const userId = currentUserId?.trim();
  const installed = new Set(Array.from(installedWorkspaceIds, (id) => id.trim()).filter(Boolean));
  const phases = activeWorkflow?.phases?.map((phase) => ({ id: phase.id })) ?? [];
  const bucketStatus = Object.fromEntries(
    bucketOrder.map((bucket) => [bucket, firstStageForBucket(bucket, phases)]),
  ) as Record<QueueBucket, string>;
  const items = cards
    .filter((card) => Boolean(userId) && text(card.assignee_user_id) === userId)
    .map((card) => {
      const status = text(card.status) ?? bucketStatus.pending;
      const workspaceId = text(card.workspace_id);
      return {
        id: text(card.id) ?? text(card.public_id) ?? "",
        publicId: text(card.public_id) ?? text(card.id) ?? "card",
        title: text(card.title) ?? text(card.name) ?? text(card.public_id) ?? "Sem titulo",
        body: text(card.body) ?? text(card.description) ?? "",
        status,
        bucket: stageBucket(status, phases),
        bucketStatus,
        priority: queuePriority(card),
        updatedAt: text(card.updated_at),
        workspaceId,
        workspaceName: text(card.workspace_name) ?? "Workspace",
        projectName: text(card.project_name),
        assigneeName: text(card.assignee_name),
        needsInstall: Boolean(workspaceId && !installed.has(workspaceId)),
        wiredTaskStatus: text(card.wired_task_status),
        wiredTaskKind: text(card.wired_task_kind),
        documentIds: Array.isArray(card.document_ids)
          ? card.document_ids.filter(
              (id): id is string => typeof id === "string" && Boolean(id.trim()),
            )
          : [],
        raw: card,
      } satisfies QueueCard;
    })
    .filter((card) => card.id)
    .sort((a, b) => {
      const priorityDiff = priorityValue(b.raw) - priorityValue(a.raw);
      if (priorityDiff !== 0) return priorityDiff;
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

export function installedWorkspaceIds(workspaces: WiredCloudWorkspace[]) {
  return workspaces.filter((workspace) => workspace.installed).map((workspace) => workspace.id);
}
