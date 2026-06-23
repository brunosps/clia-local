import type { WiredCloudCapability, WorkspaceSkill, WorkspaceSkillTarget } from "./types";

export type CapabilityStatusKind = "installed" | "pending" | "checklist";

export type CapabilityStatus = {
  capability: WiredCloudCapability;
  status: CapabilityStatusKind;
  installedTargets: WorkspaceSkillTarget[];
  missingTargets: WorkspaceSkillTarget[];
};

const TARGETS: WorkspaceSkillTarget[] = ["workspace", "codex", "claude", "copilot"];

export function deriveCapabilityStatuses(
  capabilities: WiredCloudCapability[],
  installedSkills: WorkspaceSkill[],
): CapabilityStatus[] {
  return capabilities.map((capability) => {
    const requiredTargets = normalizeCapabilityTargets(capability.targets);
    const installedTargets = installedTargetsForCapability(capability, installedSkills);
    const missingTargets = requiredTargets.filter((target) => !installedTargets.includes(target));
    return {
      capability,
      status:
        capability.auto === false ? "checklist" : missingTargets.length ? "pending" : "installed",
      installedTargets,
      missingTargets,
    };
  });
}

export function normalizeCapabilityTargets(targets: WiredCloudCapability["targets"]) {
  if (!Array.isArray(targets)) return ["workspace"] satisfies WorkspaceSkillTarget[];
  const normalized = targets.filter(isWorkspaceSkillTarget);
  return normalized.length
    ? Array.from(new Set(normalized))
    : (["workspace"] satisfies WorkspaceSkillTarget[]);
}

function installedTargetsForCapability(
  capability: WiredCloudCapability,
  installedSkills: WorkspaceSkill[],
): WorkspaceSkillTarget[] {
  const expected = skillAliases(capability.skill_id);
  const skill = installedSkills.find((item) => expected.has(item.name));
  if (!skill) return [] satisfies WorkspaceSkillTarget[];
  return Array.from(skill.installed_targets).filter(isWorkspaceSkillTarget);
}

function skillAliases(skillId: string) {
  const trimmed = skillId.trim().replace(/\/$/, "");
  const lastSegment = trimmed.split(/[/@]/).filter(Boolean).at(-1) ?? trimmed;
  return new Set([trimmed, lastSegment]);
}

function isWorkspaceSkillTarget(value: unknown): value is WorkspaceSkillTarget {
  return typeof value === "string" && TARGETS.includes(value as WorkspaceSkillTarget);
}
