import { describe, expect, it } from "vitest";
import {
  DEFAULT_WORKBENCH_SCHEMA,
  PASTEL_PALETTE,
  buildPhaseCommand,
  canAdvancePhase,
  classifyCardFields,
  groupedPhases,
  newCardFields,
  interviewPromptFor,
  mapInterviewOutputToFields,
  nextPhase,
  parseWorkbenchSchema,
  phaseById,
  phaseForStatus,
  stageDefaultOutputPolicy,
  stageInputPaths,
  stageKind,
  stageOutputPath,
} from "./workbenchSchema";
import type { WorkbenchPhase } from "./workbenchSchema";

const card = {
  id: 1,
  public_id: "DW-001",
  title: "Local PR package",
  slug: "local-pr-package",
  prd_slug: null,
};

function command(id: string, form: Record<string, unknown> = {}) {
  const phase = phaseById(DEFAULT_WORKBENCH_SCHEMA.phases, id) as WorkbenchPhase;
  return buildPhaseCommand(phase, card, form);
}

describe("default workbench schema — command parity", () => {
  it("reproduces brainstorm with modes (bare objective)", () => {
    expect(command("brainstorm", { modes: ["council", "research"] })).toBe(
      '/dw-brainstorm --mode=council+research "Local PR package"',
    );
  });

  it("reproduces brainstorm with context/output paths", () => {
    expect(
      command("brainstorm", {
        context_path: "workbench/cards/DW-001/brainstorm-input.md",
        output_path: "workbench/cards/DW-001/brainstorm.md",
      }),
    ).toBe(
      '/dw-brainstorm "Leia .dw/workbench/cards/DW-001/brainstorm-input.md antes de responder. Grave o resultado em .dw/workbench/cards/DW-001/brainstorm.md. Objetivo: Local PR package"',
    );
  });

  it("reproduces plan default and with brainstorm output hint", () => {
    expect(command("plan")).toBe('/dw-plan "Local PR package"');
    expect(command("plan", { slug: "custom-slug" })).toBe('/dw-plan "custom-slug"');
    expect(command("plan", { brainstorm_output_path: "workbench/cards/DW-001/brainstorm.md" })).toBe(
      '/dw-plan "Local PR package. Se existir, use .dw/workbench/cards/DW-001/brainstorm.md como resultado do brainstorm."',
    );
  });

  it("reproduces run resume/task/all branches", () => {
    expect(command("run", { mode: "resume" })).toBe("/dw-run --resume");
    expect(command("run", { mode: "task", task_id: "3_task" })).toBe('/dw-run "3_task"');
    expect(command("run", { mode: "all" })).toBe('/dw-run "prd-local-pr-package"');
    expect(command("run")).toBe('/dw-run "prd-local-pr-package"');
  });

  it("reproduces review modes", () => {
    expect(command("review")).toBe('/dw-review "prd-local-pr-package"');
    expect(command("review", { mode: "coverage" })).toBe(
      '/dw-review --coverage-only "prd-local-pr-package"',
    );
    expect(command("review", { mode: "code" })).toBe('/dw-review --code-only "prd-local-pr-package"');
  });

  it("reproduces qa modes", () => {
    expect(command("qa")).toBe('/dw-qa "prd-local-pr-package"');
    expect(command("qa", { mode: "fix" })).toBe('/dw-qa --fix "prd-local-pr-package"');
    expect(command("qa", { mode: "ui" })).toBe('/dw-qa --ui "prd-local-pr-package"');
    expect(command("qa", { mode: "default" })).toBe('/dw-qa "prd-local-pr-package"');
  });

  it("reproduces terminal commands and the new security phase", () => {
    expect(command("security")).toBe("/dw-secure-audit");
    expect(command("commit")).toBe("/dw-commit");
    expect(command("local-pr")).toBe("/dw-generate-pr");
    expect(command("done")).toBe("");
  });

  it("places security between qa and commit (Backlog is the shared Intake, not a phase)", () => {
    const ids = DEFAULT_WORKBENCH_SCHEMA.phases.map((phase) => phase.id);
    expect(ids).toEqual([
      "brainstorm",
      "plan",
      "run",
      "review",
      "qa",
      "security",
      "commit",
      "local-pr",
      "done",
    ]);
    expect(nextPhase(DEFAULT_WORKBENCH_SCHEMA.phases, "qa")?.id).toBe("security");
    expect(nextPhase(DEFAULT_WORKBENCH_SCHEMA.phases, "security")?.id).toBe("commit");
  });
});

describe("phase lookup", () => {
  it("maps status to phase with first-phase fallback", () => {
    expect(phaseForStatus(DEFAULT_WORKBENCH_SCHEMA.phases, "qa").id).toBe("qa");
    expect(phaseForStatus(DEFAULT_WORKBENCH_SCHEMA.phases, "security").id).toBe("security");
    expect(phaseForStatus(DEFAULT_WORKBENCH_SCHEMA.phases, "unknown-status").id).toBe("brainstorm");
  });
});

describe("advance gate", () => {
  const phase: WorkbenchPhase = {
    id: "p",
    label: "P",
    status: "p",
    description: "",
    fields: [{ key: "context", label: "Contexto", type: "textarea" }],
    action: { type: "none" },
    advance: { requireFields: ["context"], artifact: "x.md", expectJson: ["status"] },
  };

  it("blocks until all conditions pass (AND)", () => {
    const blocked = canAdvancePhase(phase, { form: {}, artifactExists: false, agentJson: null });
    expect(blocked.ok).toBe(false);
    expect(blocked.reasons).toHaveLength(3);

    const passed = canAdvancePhase(phase, {
      form: { context: "algo" },
      artifactExists: true,
      agentJson: { status: "done" },
    });
    expect(passed.ok).toBe(true);
    expect(passed.reasons).toHaveLength(0);
  });

  it("treats no advance config as always allowed", () => {
    expect(canAdvancePhase({ ...phase, advance: undefined }, { form: {} }).ok).toBe(true);
  });

  it("gates on a required output document", () => {
    const planning: WorkbenchPhase = {
      id: "plan",
      label: "Plan",
      status: "planned",
      description: "",
      fields: [],
      action: { type: "command", base: "/dw-plan" },
      output: { path: "spec/{{card.slug}}/prd.md", policy: "required", capture: true },
    };
    expect(canAdvancePhase(planning, { form: {}, outputExists: false }).ok).toBe(false);
    expect(canAdvancePhase(planning, { form: {}, outputExists: true }).ok).toBe(true);
  });

  it("requires human approval on approval stages, and override bypasses any gate", () => {
    const approval: WorkbenchPhase = {
      id: "approve",
      label: "Aprovar plano",
      status: "approval",
      description: "",
      fields: [],
      action: { type: "none" },
      kind: "approval",
    };
    expect(canAdvancePhase(approval, { form: {} }).ok).toBe(false);
    expect(canAdvancePhase(approval, { form: {}, approved: true }).ok).toBe(true);
    expect(canAdvancePhase(approval, { form: {}, override: true }).ok).toBe(true);
  });
});

describe("stage model helpers", () => {
  function phase(extra: Partial<WorkbenchPhase>): WorkbenchPhase {
    return {
      id: "x",
      label: "X",
      status: "x",
      description: "",
      fields: [],
      action: { type: "none" },
      ...extra,
    };
  }

  it("derives the stage kind from action/group when not declared", () => {
    expect(stageKind(phase({ action: { type: "none" } }))).toBe("status");
    expect(stageKind(phase({ group: "planejamento", action: { type: "command", base: "/x" } }))).toBe(
      "planning",
    );
    expect(stageKind(phase({ group: "entrega", action: { type: "command", base: "/x" } }))).toBe(
      "delivery",
    );
    expect(stageKind(phase({ group: "execucao", action: { type: "command", base: "/x" } }))).toBe(
      "execution",
    );
    expect(stageKind(phase({ kind: "review", action: { type: "none" } }))).toBe("review");
  });

  it("maps default output policy by kind", () => {
    expect(stageDefaultOutputPolicy("planning")).toBe("required");
    expect(stageDefaultOutputPolicy("execution")).toBe("optional");
    expect(stageDefaultOutputPolicy("status")).toBe("none");
  });

  it("resolves input/output path templates", () => {
    const p = phase({
      inputs: [{ path: "spec/{{card.slug}}/brainstorm.md" }, { path: "notes.md", required: false }],
      output: { path: "spec/{{card.slug}}/prd.md", policy: "required" },
    });
    const inputs = stageInputPaths(p, card);
    expect(inputs[0]).toEqual({ path: "spec/local-pr-package/brainstorm.md", required: true });
    expect(inputs[1].required).toBe(false);
    expect(stageOutputPath(p, card)).toBe("spec/local-pr-package/prd.md");
    expect(stageOutputPath(phase({ output: { path: "x", policy: "none" } }), card)).toBeNull();
  });
});

describe("interview helpers", () => {
  const interviewPhase: WorkbenchPhase = {
    id: "refine",
    label: "Refinar",
    status: "draft",
    description: "",
    fields: [
      { key: "context", label: "Contexto", type: "textarea" },
      { key: "expected_result", label: "Resultado", type: "textarea" },
      { key: "checklist", label: "Checklist", type: "checklist" },
    ],
    action: {
      type: "interview",
      style: "horizons",
      minQuestions: 5,
      maxQuestions: 9,
      fillsFields: ["description", "context", "expected_result", "checklist"],
    },
  };

  it("builds a prompt listing the phase's declared fields and respecting limits", () => {
    const prompt = interviewPromptFor(interviewPhase, ["Card: DW-001"]);
    expect(prompt).toContain("H1");
    expect(prompt).toContain("description, context, expected_result, checklist");
    expect(prompt).toContain("no máximo 9");
    expect(prompt).toContain("Card: DW-001");
  });

  it("maps final JSON onto declared field values, coercing arrays", () => {
    const mapped = mapInterviewOutputToFields(interviewPhase, {
      description: "desc",
      context: "ctx",
      expected_result: "res",
      checklist: ["a", "b", ""],
    });
    expect(mapped.context).toBe("ctx");
    expect(mapped.checklist).toEqual(["a", "b"]);
  });
});

describe("new-card form (newCard block)", () => {
  it("default newCard is simplified to prompt/projects/attachments; first phase is brainstorm", () => {
    const classified = classifyCardFields(newCardFields(DEFAULT_WORKBENCH_SCHEMA));
    // Simplified intake: no separate title field; the prompt is bound to body.
    expect(classified.titleField).toBeUndefined();
    expect(classified.bodyField?.key).toBe("prompt");
    expect(classified.bodyField?.binding).toBe("body");
    expect(classified.bodyField?.required).toBe(true);
    expect(classified.projectsField?.type).toBe("projects");
    expect(classified.attachmentsField?.type).toBe("attachments");
    expect(classified.customFields).toHaveLength(0);

    // Backlog is the shared Intake now, so the flow starts at brainstorm.
    expect(phaseById(DEFAULT_WORKBENCH_SCHEMA.phases, "backlog")).toBeUndefined();
    expect(DEFAULT_WORKBENCH_SCHEMA.phases[0].id).toBe("brainstorm");
  });

  it("parses a custom newCard block with projects/attachments + extra fields", () => {
    const result = parseWorkbenchSchema({
      version: 1,
      newCard: {
        fields: [
          { key: "title", label: "T", type: "text", binding: "title", required: true },
          { key: "projects", label: "P", type: "projects" },
          { key: "attachments", label: "A", type: "attachments" },
          { key: "priority", label: "Prioridade", type: "select", options: [{ value: "hi", label: "Alta" }] },
        ],
      },
      phases: [{ id: "backlog", label: "B", status: "draft", action: { type: "none" } }],
    });
    expect(result.usedDefault).toBe(false);
    const classified = classifyCardFields(newCardFields(result.schema));
    expect(classified.titleField?.binding).toBe("title");
    expect(classified.projectsField?.type).toBe("projects");
    expect(classified.customFields.map((f) => f.key)).toEqual(["priority"]);
  });

  it("falls back to the first phase fields when no newCard block is defined", () => {
    const result = parseWorkbenchSchema({
      version: 1,
      phases: [
        {
          id: "backlog",
          label: "B",
          status: "draft",
          fields: [{ key: "title", label: "T", type: "text", binding: "title" }],
          action: { type: "none" },
        },
      ],
    });
    expect(newCardFields(result.schema).map((f) => f.key)).toEqual(["title"]);
  });
});

describe("phase groups", () => {
  it("groups the default phases into 4 ordered bands", () => {
    const bands = groupedPhases(DEFAULT_WORKBENCH_SCHEMA);
    expect(bands.map((b) => b.group.id)).toEqual([
      "planejamento",
      "execucao",
      "entrega",
      "concluido",
    ]);
    expect(bands[0].phases.map((p) => p.id)).toEqual(["brainstorm", "plan"]);
    expect(bands[1].phases.map((p) => p.id)).toEqual(["run", "review", "qa", "security"]);
    expect(bands[2].phases.map((p) => p.id)).toEqual(["commit", "local-pr"]);
    expect(bands[3].phases.map((p) => p.id)).toEqual(["done"]);
    expect(bands[0].color).toBe(PASTEL_PALETTE[0]);
  });

  it("puts ungrouped phases into a trailing 'Outros' band", () => {
    const result = parseWorkbenchSchema({
      version: 1,
      groups: [{ id: "g1", label: "G1", color: "#abcdef" }],
      phases: [
        { id: "a", label: "A", status: "a", group: "g1", action: { type: "none" } },
        { id: "b", label: "B", status: "b", action: { type: "none" } },
      ],
    });
    const bands = groupedPhases(result.schema);
    expect(bands.map((b) => b.group.id)).toEqual(["g1", "_ungrouped"]);
    expect(bands[0].color).toBe("#abcdef"); // honors schema color
    expect(bands[1].phases.map((p) => p.id)).toEqual(["b"]);
  });

  it("sets wip limits on the execução phases and parses wipLimit", () => {
    const qa = phaseById(DEFAULT_WORKBENCH_SCHEMA.phases, "qa") as WorkbenchPhase;
    expect(qa.wipLimit).toBe(2);
    const run = phaseById(DEFAULT_WORKBENCH_SCHEMA.phases, "run") as WorkbenchPhase;
    expect(run.wipLimit).toBe(1);

    const result = parseWorkbenchSchema({
      version: 1,
      phases: [
        { id: "a", label: "A", status: "a", wipLimit: 5, action: { type: "none" } },
        { id: "b", label: "B", status: "b", wipLimit: 0, action: { type: "none" } },
        { id: "c", label: "C", status: "c", wipLimit: -3, action: { type: "none" } },
      ],
    });
    expect(result.schema.phases[0].wipLimit).toBe(5);
    expect(result.schema.phases[1].wipLimit).toBeUndefined();
    expect(result.schema.phases[2].wipLimit).toBeUndefined();
  });

  it("returns a single unlabeled band when no groups are defined", () => {
    const result = parseWorkbenchSchema({
      version: 1,
      phases: [{ id: "a", label: "A", status: "a", action: { type: "none" } }],
    });
    const bands = groupedPhases(result.schema);
    expect(bands).toHaveLength(1);
    expect(bands[0].group.label).toBe("");
  });
});

describe("schema parsing", () => {
  it("falls back to default on bad JSON or empty phases", () => {
    expect(parseWorkbenchSchema("{not json").usedDefault).toBe(true);
    expect(parseWorkbenchSchema({ version: 1, phases: [] }).usedDefault).toBe(true);
  });

  it("parses a valid schema and flags duplicate ids + dangling field refs", () => {
    const result = parseWorkbenchSchema({
      version: 1,
      phases: [
        {
          id: "a",
          label: "A",
          status: "a",
          fields: [{ key: "x", label: "X", type: "text" }],
          action: { type: "none" },
          advance: { requireFields: ["x", "missing"] },
        },
        { id: "a", label: "Dup", status: "dup", action: { type: "none" } },
      ],
    });
    expect(result.usedDefault).toBe(false);
    expect(result.schema.phases).toHaveLength(1);
    expect(result.warnings.some((w) => w.includes("duplicado"))).toBe(true);
    expect(result.warnings.some((w) => w.includes("missing"))).toBe(true);
  });
});
