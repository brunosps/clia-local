import { describe, expect, it } from "vitest";
import { buildQueue, installedWorkspaceIds } from "./queue";
import type { WiredCloudCard, WiredCloudWorkspace } from "./types";
import type { WorkbenchSchema } from "./workbenchSchema";

const workflow = {
  phases: [
    { id: "backlog", label: "Backlog", status: "ready", description: "", fields: [], action: { type: "none" } },
    { id: "run", label: "Run", status: "ready", description: "", fields: [], action: { type: "none" } },
    { id: "qa", label: "QA", status: "ready", description: "", fields: [], action: { type: "none" } },
    { id: "done", label: "Done", status: "ready", description: "", fields: [], action: { type: "none" } },
  ],
} satisfies Pick<WorkbenchSchema, "phases">;

function card(patch: Partial<WiredCloudCard>): WiredCloudCard {
  return {
    id: "card-1",
    public_id: "APP-1",
    title: "Implementar fila",
    status: "backlog",
    priority: "medium",
    updated_at: "2026-06-01T10:00:00Z",
    workspace_id: "workspace-1",
    workspace_name: "Core",
    assignee_user_id: "user-1",
    ...patch,
  };
}

describe("buildQueue", () => {
  it("keeps exactly assigned cards ordered by priority and updated_at", () => {
    const queue = buildQueue(
      [
        card({ id: "low", public_id: "APP-3", priority: "low", updated_at: "2026-06-03T10:00:00Z" }),
        card({ id: "other", public_id: "APP-4", assignee_user_id: "user-2", priority: "high" }),
        card({ id: "high-old", public_id: "APP-1", priority: "high", updated_at: "2026-06-01T10:00:00Z" }),
        card({ id: "high-new", public_id: "APP-2", priority: "high", updated_at: "2026-06-02T10:00:00Z" }),
      ],
      "user-1",
      ["workspace-1"],
      workflow,
    );

    expect(queue.items.map((item) => item.id)).toEqual(["high-new", "high-old", "low"]);
  });

  it("marks install needs and bucketizes by workflow status", () => {
    const queue = buildQueue(
      [
        card({ id: "doing", status: "run", workspace_id: "workspace-2" }),
        card({ id: "validating", status: "qa", workspace_id: "workspace-1" }),
        card({ id: "done", status: "done", workspace_id: "workspace-1" }),
      ],
      "user-1",
      ["workspace-1"],
      workflow,
    );

    expect(queue.buckets.doing.map((item) => item.id)).toEqual(["doing"]);
    expect(queue.buckets.validating.map((item) => item.id)).toEqual(["validating"]);
    expect(queue.buckets.done.map((item) => item.id)).toEqual(["done"]);
    expect(queue.items.find((item) => item.id === "doing")?.needsInstall).toBe(true);
    expect(queue.items.find((item) => item.id === "validating")?.needsInstall).toBe(false);
  });
});

describe("installedWorkspaceIds", () => {
  it("returns cloud ids for installed workspaces", () => {
    const workspaces: WiredCloudWorkspace[] = [
      { id: "one", name: "One", installed: true },
      { id: "two", name: "Two", installed: false },
      { id: "three", name: "Three", installed: true },
    ];

    expect(installedWorkspaceIds(workspaces)).toEqual(["one", "three"]);
  });
});
