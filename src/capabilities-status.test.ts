import { describe, expect, it } from "vitest";

import { deriveCapabilityStatuses } from "./capabilities-status";
import type { WiredCloudCapability, WorkspaceSkill } from "./types";

const baseSkill: WorkspaceSkill = {
  name: "reviewer",
  source: "workspace",
  installed_targets: ["workspace", "codex"],
  file_count: 1,
  byte_count: 10,
};

function capability(input: Partial<WiredCloudCapability>): WiredCloudCapability {
  return {
    id: input.id ?? "cap-1",
    workspace_id: "w1",
    skill_id: input.skill_id ?? "org/reviewer",
    label: input.label ?? "Reviewer",
    targets: input.targets ?? ["workspace", "codex"],
    auto: input.auto ?? true,
    install_spec: input.install_spec ?? {},
  };
}

describe("deriveCapabilityStatuses", () => {
  it("marks an auto capability installed when all manifest targets are installed", () => {
    const [status] = deriveCapabilityStatuses([capability({})], [baseSkill]);

    expect(status.status).toBe("installed");
    expect(status.missingTargets).toEqual([]);
  });

  it("marks auto capabilities pending when a target is missing", () => {
    const [status] = deriveCapabilityStatuses(
      [capability({ targets: ["workspace", "claude"] })],
      [baseSkill],
    );

    expect(status.status).toBe("pending");
    expect(status.missingTargets).toEqual(["claude"]);
  });

  it("keeps manual capabilities as checklist even if the skill exists", () => {
    const [status] = deriveCapabilityStatuses([capability({ auto: false })], [baseSkill]);

    expect(status.status).toBe("checklist");
  });
});
