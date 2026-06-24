import { describe, expect, it } from "vitest";
import { buildQueue, statusBucket } from "./queue";
import type { Project, RequirementCard } from "./types";

function card(patch: Partial<RequirementCard>): RequirementCard {
  return {
    id: 1,
    workspace_id: 1,
    project_id: 1,
    project_ids: [1],
    public_id: "APP-1",
    title: "Implementar fila",
    slug: "implementar-fila",
    body: "",
    priority: "medium",
    checklist_json: "[]",
    agent_prompt: "",
    status: "todo",
    created_at: "2026-06-01T10:00:00Z",
    updated_at: "2026-06-01T10:00:00Z",
    ...patch,
  };
}

const projects: Project[] = [
  { id: 1, workspace_id: 1, name: "App", path: "/app", created_at: "2026-06-01T00:00:00Z" },
  { id: 2, workspace_id: 1, name: "Api", path: "/api", created_at: "2026-06-01T00:00:00Z" },
];

describe("buildQueue", () => {
  it("orders by priority then updated_at and drops archived cards", () => {
    const queue = buildQueue(
      [
        card({ id: 1, priority: "low", updated_at: "2026-06-03T10:00:00Z" }),
        card({ id: 2, priority: "high", updated_at: "2026-06-01T10:00:00Z" }),
        card({ id: 3, priority: "high", updated_at: "2026-06-02T10:00:00Z" }),
        card({ id: 4, priority: "high", status: "archived" }),
      ],
      projects,
    );

    expect(queue.items.map((item) => item.id)).toEqual(["3", "2", "1"]);
  });

  it("buckets by status and counts checklist progress", () => {
    const queue = buildQueue(
      [
        card({ id: 1, status: "doing" }),
        card({ id: 2, status: "qa" }),
        card({
          id: 3,
          status: "done",
          checklist_json: JSON.stringify([
            { id: "a", text: "x", done: true },
            { id: "b", text: "y", done: false },
          ]),
        }),
      ],
      projects,
    );

    expect(queue.buckets.doing.map((item) => item.id)).toEqual(["1"]);
    expect(queue.buckets.validating.map((item) => item.id)).toEqual(["2"]);
    expect(queue.buckets.done.map((item) => item.id)).toEqual(["3"]);
    const done = queue.buckets.done[0];
    expect(done.checklistTotal).toBe(2);
    expect(done.checklistDone).toBe(1);
  });

  it("filters by project and resolves project names", () => {
    const queue = buildQueue(
      [
        card({ id: 1, project_ids: [1] }),
        card({ id: 2, project_ids: [2] }),
        card({ id: 3, project_ids: [1, 2] }),
      ],
      projects,
      { projectId: 2 },
    );

    expect(queue.items.map((item) => item.id).sort()).toEqual(["2", "3"]);
    expect(queue.items.find((item) => item.id === "3")?.projectNames).toEqual(["App", "Api"]);
  });
});

describe("statusBucket", () => {
  it("maps legacy and canonical statuses to the four columns", () => {
    expect(statusBucket("draft")).toBe("pending");
    expect(statusBucket("todo")).toBe("pending");
    expect(statusBucket("running")).toBe("doing");
    expect(statusBucket("reviewing")).toBe("validating");
    expect(statusBucket("done")).toBe("done");
    expect(statusBucket("anything-else")).toBe("pending");
  });
});
