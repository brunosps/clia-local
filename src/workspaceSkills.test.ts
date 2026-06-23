import { describe, expect, it } from "vitest";
import {
  applySkillAutocomplete,
  composeSkillPrompt,
  filterSkillSuggestions,
  groupSkillSuggestions,
  isSkillSlashCommand,
  resolveSkillSlashCommand,
  skillAutocompleteQuery,
  workspaceSkillFilePath,
} from "./workspaceSkills";
import type { WorkspaceSkill } from "./types";

const skills: WorkspaceSkill[] = [
  {
    name: "dw-ui-discipline",
    description: "UI",
    source: "workspace",
    scope: "workspace",
    scope_label: "Workspace",
    path: ".dw/gui/skills/dw-ui-discipline/SKILL.md",
    bundled: true,
    owner: "dw-redesign-ui",
    kind: "protocol",
    group: "dev-workflow",
    installed_targets: ["workspace"],
    file_count: 1,
    byte_count: 100,
  },
  {
    name: "custom-review",
    description: "Custom",
    source: "workspace",
    scope: "workspace",
    scope_label: "Workspace",
    group: "Avulsas",
    installed_targets: ["workspace"],
    file_count: 1,
    byte_count: 100,
  },
  {
    name: "api-testing-recipes",
    description: "API tests owned by dw-qa",
    source: "workspace",
    scope: "workspace",
    scope_label: "Workspace",
    bundled: true,
    owner: "dw-qa",
    kind: "recipe-pack",
    group: "dev-workflow",
    installed_targets: ["workspace"],
    file_count: 1,
    byte_count: 100,
  },
  {
    name: "dw-local-without-registry",
    description: "DW skill without registry metadata",
    source: "workspace",
    scope: "workspace",
    scope_label: "Workspace",
    group: "Avulsas",
    installed_targets: ["workspace"],
    file_count: 1,
    byte_count: 100,
  },
  {
    name: "speckit.specify",
    description: "Specify a feature",
    source: "workspace",
    scope: "workspace",
    scope_label: "Workspace",
    bundled: true,
    group: "GitHub spec-kit",
    installed_targets: ["workspace"],
    file_count: 1,
    byte_count: 100,
  },
  {
    name: "shared-skill",
    description: "Workspace shared",
    source: "workspace",
    scope: "workspace",
    scope_label: "Workspace",
    group: "Avulsas",
    priority: 1,
    installed_targets: ["workspace"],
    file_count: 1,
    byte_count: 100,
  },
  {
    name: "shared-skill",
    description: "Project shared",
    source: "project:demo",
    scope: "project",
    scope_label: "Projeto: demo",
    group: "Avulsas",
    priority: 0,
    installed_targets: ["codex"],
    file_count: 1,
    byte_count: 100,
  },
];

describe("workspace skill slash command", () => {
  it("detects direct skill slash commands and legacy /skill commands", () => {
    expect(isSkillSlashCommand(" /skill dw-ui-discipline revise")).toBe(true);
    expect(isSkillSlashCommand("/dw-ui-discipline revise")).toBe(true);
    expect(isSkillSlashCommand("texto /skill")).toBe(false);
  });

  it("resolves direct skill name and request", () => {
    const result = resolveSkillSlashCommand("/dw-ui-discipline revise o layout", skills);
    expect(result?.ok).toBe(true);
    if (result?.ok) {
      expect(result.skill.name).toBe("dw-ui-discipline");
      expect(result.request).toBe("revise o layout");
    }
  });

  it("keeps legacy /skill supported", () => {
    const result = resolveSkillSlashCommand("/skill $dw-ui-discipline revise o layout", skills);
    expect(result?.ok).toBe(true);
    if (result?.ok) {
      expect(result.skill.name).toBe("dw-ui-discipline");
      expect(result.request).toBe("revise o layout");
    }
  });

  it("uses a default request when direct command has no text", () => {
    const result = resolveSkillSlashCommand("/dw-ui-discipline", skills);
    expect(result?.ok).toBe(true);
    if (result?.ok) {
      expect(result.request).toContain("Execute esta skill");
    }
  });

  it("returns clear errors for missing legacy data", () => {
    expect(resolveSkillSlashCommand("/skill", skills)).toEqual({
      ok: false,
      error: "Informe a skill após /skill.",
    });
    expect(resolveSkillSlashCommand("/skill nope faça algo", skills)).toEqual({
      ok: false,
      error: "Skill não encontrada no workspace: nope",
    });
    expect(resolveSkillSlashCommand("/nope faça algo", skills)).toBeNull();
  });

  it("composes an injected prompt without requiring filesystem discovery", () => {
    const prompt = composeSkillPrompt(skills[0], "---\nname: dw-ui-discipline\n---", "revise");
    expect(prompt).toContain('Use a skill de workspace "dw-ui-discipline"');
    expect(prompt).toContain("Não procure essa skill no filesystem");
    expect(prompt).toContain("Pedido do usuário:\nrevise");
  });

  it("resolves skill file path from workspace root", () => {
    expect(workspaceSkillFilePath("/tmp/wks", skills[0])).toBe(
      "/tmp/wks/.dw/gui/skills/dw-ui-discipline/SKILL.md",
    );
  });

  it("keeps absolute skill paths discovered from external projects", () => {
    expect(
      workspaceSkillFilePath("/tmp/wks", {
        ...skills[0],
        path: "/tmp/project/.agents/skills/dw-ui-discipline/SKILL.md",
      }),
    ).toBe("/tmp/project/.agents/skills/dw-ui-discipline/SKILL.md");
  });

  it("extracts autocomplete query only while editing the slash command", () => {
    expect(skillAutocompleteQuery("/")).toBe("");
    expect(skillAutocompleteQuery("/dw")).toBe("dw");
    expect(skillAutocompleteQuery("/skill")).toBe("");
    expect(skillAutocompleteQuery("/skill dw-ui")).toBe("dw-ui");
    expect(skillAutocompleteQuery("/skill dw-ui revise")).toBeNull();
    expect(skillAutocompleteQuery("/dw-ui-discipline revise")).toBeNull();
    expect(skillAutocompleteQuery("texto /skill")).toBeNull();
  });

  it("applies autocomplete by replacing the current slash command", () => {
    expect(applySkillAutocomplete("/", "dw-ui-discipline")).toBe("/dw-ui-discipline ");
    expect(applySkillAutocomplete("  /skill dw", "dw-ui-discipline")).toBe("  /dw-ui-discipline ");
  });

  it("groups bundled suggestions before standalone skills", () => {
    expect(
      groupSkillSuggestions(skills)
        .filter((group) => group.scopeLabel === "Workspace")
        .map((group) => group.frameworkLabel),
    ).toEqual(["dev-workflow", "GitHub spec-kit", "Avulsas"]);
  });

  it("filters autocomplete by skill name without matching bundled group or owner", () => {
    expect(filterSkillSuggestions(skills, "dw").map((skill) => skill.name)).toEqual([
      "dw-local-without-registry",
      "dw-ui-discipline",
    ]);
    expect(filterSkillSuggestions(skills, "api").map((skill) => skill.name)).toEqual([
      "api-testing-recipes",
    ]);
    expect(filterSkillSuggestions(skills, "github spec kit").map((skill) => skill.name)).toEqual([
      "speckit.specify",
    ]);
  });

  it("resolves duplicate skill names by project before workspace", () => {
    const result = resolveSkillSlashCommand("/shared-skill execute", skills);
    expect(result?.ok).toBe(true);
    if (result?.ok) {
      expect(result.skill.scope).toBe("project");
    }
  });

  it("does not guess framework group from skill name without metadata", () => {
    const avulsasGroup = groupSkillSuggestions(skills).find(
      (group) => group.scopeLabel === "Workspace" && group.frameworkLabel === "Avulsas",
    );
    expect(avulsasGroup?.skills.map((skill) => skill.name)).toContain("dw-local-without-registry");
  });
});
