import { describe, expect, it } from "vitest";
import { allWiredTaskStatusViews, wiredTaskStatusView } from "./task-status";

describe("wiredTaskStatusView", () => {
  it("defines label and action for every discrete task status", () => {
    expect(allWiredTaskStatusViews().map((view) => view.status)).toEqual([
      "pending",
      "awaiting_worker",
      "dispatched",
      "running",
      "done",
      "failed",
    ]);

    for (const view of allWiredTaskStatusViews()) {
      expect(view.label).toBeTruthy();
      expect(view.tone).toBeTruthy();
    }

    expect(wiredTaskStatusView("awaiting_worker")).toMatchObject({
      label: "aguardando worker",
      action: null,
    });
    expect(wiredTaskStatusView("failed")).toMatchObject({
      label: "falhou",
      action: "re-disparar",
    });
  });

  it("maps legacy statuses without expanding the card state machine", () => {
    expect(wiredTaskStatusView("waiting_approval")?.status).toBe("pending");
    expect(wiredTaskStatusView("approved")?.status).toBe("dispatched");
    expect(wiredTaskStatusView("cancelled")?.status).toBe("failed");
  });
});
