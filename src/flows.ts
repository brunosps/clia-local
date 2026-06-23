import {
  DEFAULT_WORKBENCH_SCHEMA,
  parseWorkbenchSchema,
  type WorkbenchField,
  type WorkbenchFieldType,
  type WorkbenchSchema,
} from "./workbenchSchema";

// ---------------------------------------------------------------------------
// Flow registry — several named workbench schemas coexisting in one workspace.
// Each flow is a `.dw/flows/<id>.json` file (shape = WorkbenchSchema); the
// `.dw/flows/index.json` file lists them and names the default + intake form.
// All flows SHARE git, agents, projects and evidence; only the pipeline differs.
// ---------------------------------------------------------------------------

export type FlowMeta = {
  id: string;
  label: string;
  description?: string;
  /** Bundled preset this flow was applied from, if any. */
  preset?: string;
  /** Command that analyzes the repo to generate context (e.g. /dw-analyze-project). */
  analyzeCommand?: string;
  /** Project-relative path that, when present, means the project is already analyzed. */
  analyzeMarker?: string;
  /** Command that scouts candidate opportunities (e.g. /dw-opportunities). */
  suggestCommand?: string;
};

export type FlowIndex = {
  flows: FlowMeta[];
  default?: string;
  /** Generic, flow-agnostic fields for the "new card" (intake) form. */
  intake?: { fields: WorkbenchField[] };
};

export type FlowRegistry = {
  /** Ordered metadata for the flow switcher. */
  flows: FlowMeta[];
  /** Parsed schema per flow id. */
  schemas: Record<string, WorkbenchSchema>;
  defaultFlowId: string;
  /** Fields for the shared intake/new-card form. */
  intakeFields: WorkbenchField[];
};

/** Built-in intake form: flow-agnostic, used until a flow is chosen. */
export const DEFAULT_INTAKE_FIELDS: WorkbenchField[] = [
  {
    key: "prompt",
    label: "Prompt / contexto / descrição",
    type: "textarea",
    binding: "body",
    required: true,
    placeholder:
      "Descreva o que precisa: objetivo, contexto, restrições. O título do card sai daqui.",
  },
  { key: "projects", label: "Projetos vinculados", type: "projects" },
  { key: "attachments", label: "Anexos", type: "attachments" },
];

// ---------------------------------------------------------------------------
// spec-kit preset — /speckit.* pipeline. Proves the schema is framework-agnostic:
// buildPhaseCommand emits `/speckit.specify "..."` with no code change.
// ---------------------------------------------------------------------------

export const PRESET_SPEC_KIT: WorkbenchSchema = {
  version: 1,
  newCard: { fields: DEFAULT_INTAKE_FIELDS },
  groups: [
    { id: "sk-specify", label: "Specify" },
    { id: "sk-plan", label: "Plan" },
    { id: "sk-build", label: "Implement" },
  ],
  phases: [
    {
      id: "sk-constitution",
      label: "Constitution",
      status: "sk_constitution",
      group: "sk-specify",
      description: "Princípios do projeto (uma vez). Gera .specify/memory/constitution.md.",
      fields: [{ key: "principles", label: "Princípios", type: "textarea" }],
      action: {
        type: "command",
        base: "/speckit.constitution",
        promptParts: [{ template: "{{field.principles|default:card.title}}" }],
      },
      advance: {},
    },
    {
      id: "sk-specify",
      label: "Specify",
      status: "sk_specify",
      group: "sk-specify",
      description: "Descrição da feature → specs/NNN/spec.md.",
      fields: [{ key: "feature", label: "Descrição da feature", type: "textarea", required: true }],
      action: {
        type: "command",
        base: "/speckit.specify",
        promptParts: [{ template: "{{field.feature|default:card.title}}" }],
      },
      advance: { requireFields: ["feature"] },
    },
    {
      id: "sk-clarify",
      label: "Clarify",
      status: "sk_clarify",
      group: "sk-specify",
      description: "Resolve ambiguidades do spec antes de planejar.",
      fields: [],
      action: { type: "command", base: "/speckit.clarify" },
      advance: {},
    },
    {
      id: "sk-plan",
      label: "Plan",
      status: "sk_plan",
      group: "sk-plan",
      description: "Decisões técnicas e arquitetura → plan.md.",
      fields: [{ key: "constraints", label: "Restrições técnicas", type: "textarea" }],
      action: {
        type: "command",
        base: "/speckit.plan",
        promptParts: [
          { template: "{{field.constraints}}", when: [{ field: "constraints", nonEmpty: true }] },
        ],
      },
      advance: {},
    },
    {
      id: "sk-tasks",
      label: "Tasks",
      status: "sk_tasks",
      group: "sk-plan",
      description: "Quebra o plano em tasks acionáveis → tasks.md.",
      fields: [],
      action: { type: "command", base: "/speckit.tasks" },
      advance: {},
    },
    {
      id: "sk-analyze",
      label: "Analyze",
      status: "sk_analyze",
      group: "sk-build",
      description: "Gate opcional de consistência entre spec, plan e tasks.",
      fields: [],
      action: { type: "command", base: "/speckit.analyze" },
      advance: {},
    },
    {
      id: "sk-implement",
      label: "Implement",
      status: "sk_implement",
      group: "sk-build",
      wipLimit: 1,
      description: "Executa as tasks e implementa a feature.",
      fields: [],
      action: { type: "command", base: "/speckit.implement" },
      advance: {},
    },
  ],
};

// ---------------------------------------------------------------------------
// OpenSpec preset — /opsx:* change-proposal pipeline.
// ---------------------------------------------------------------------------

export const PRESET_OPENSPEC: WorkbenchSchema = {
  version: 1,
  newCard: { fields: DEFAULT_INTAKE_FIELDS },
  groups: [
    { id: "os-plan", label: "Propose" },
    { id: "os-build", label: "Apply" },
    { id: "os-finalize", label: "Finalize" },
  ],
  phases: [
    {
      id: "os-propose",
      label: "Propose",
      status: "os_propose",
      group: "os-plan",
      description: "Cria a change proposal (proposal.md, specs/, design.md, tasks.md).",
      fields: [{ key: "change", label: "Descrição da mudança", type: "textarea", required: true }],
      action: {
        type: "command",
        base: "/opsx:propose",
        promptParts: [{ template: "{{field.change|default:card.title}}" }],
      },
      advance: { requireFields: ["change"] },
    },
    {
      id: "os-apply",
      label: "Apply",
      status: "os_apply",
      group: "os-build",
      wipLimit: 1,
      description: "Implementa a change a partir das tasks.",
      fields: [],
      action: { type: "command", base: "/opsx:apply" },
      advance: {},
    },
    {
      id: "os-verify",
      label: "Verify",
      status: "os_verify",
      group: "os-build",
      description: "Verifica completude, correção e coerência da change.",
      fields: [],
      action: { type: "command", base: "/opsx:verify" },
      advance: {},
    },
    {
      id: "os-sync",
      label: "Sync",
      status: "os_sync",
      group: "os-finalize",
      description: "Mescla os delta specs em openspec/specs/.",
      fields: [],
      action: { type: "command", base: "/opsx:sync" },
      advance: {},
    },
    {
      id: "os-archive",
      label: "Archive",
      status: "os_archive",
      group: "os-finalize",
      description: "Arquiva a change concluída.",
      fields: [],
      action: { type: "command", base: "/opsx:archive" },
      advance: {},
    },
  ],
};

// ---------------------------------------------------------------------------
// BMAD-METHOD preset — agent-persona phases (Analyst → PM → Architect → Dev).
// ---------------------------------------------------------------------------

export const PRESET_BMAD: WorkbenchSchema = {
  version: 1,
  newCard: { fields: DEFAULT_INTAKE_FIELDS },
  groups: [
    { id: "bmad-analysis", label: "Analysis" },
    { id: "bmad-planning", label: "Planning" },
    { id: "bmad-solution", label: "Solutioning" },
    { id: "bmad-impl", label: "Implementation" },
  ],
  phases: [
    {
      id: "bmad-brief",
      label: "Product brief",
      status: "bmad_brief",
      group: "bmad-analysis",
      description: "Analyst: discovery + product brief.",
      fields: [{ key: "scope", label: "Escopo / pesquisa", type: "textarea" }],
      action: {
        type: "command",
        base: "/bmad-product-brief",
        promptParts: [{ template: "{{field.scope|default:card.title}}" }],
      },
      advance: {},
    },
    {
      id: "bmad-prd",
      label: "PRD",
      status: "bmad_prd",
      group: "bmad-planning",
      description: "PM: requirements & PRD.",
      fields: [],
      action: {
        type: "command",
        base: "/bmad-prd",
        promptParts: [{ template: "{{card.title}}" }],
      },
      advance: {},
    },
    {
      id: "bmad-ux",
      label: "UX",
      status: "bmad_ux",
      group: "bmad-planning",
      description: "UX Designer: DESIGN.md + EXPERIENCE.md.",
      fields: [],
      action: { type: "command", base: "/bmad-ux" },
      advance: {},
    },
    {
      id: "bmad-arch",
      label: "Architecture",
      status: "bmad_arch",
      group: "bmad-solution",
      description: "Architect: architecture.md com ADRs.",
      fields: [],
      action: { type: "command", base: "/bmad-create-architecture" },
      advance: {},
    },
    {
      id: "bmad-stories",
      label: "Epics & stories",
      status: "bmad_stories",
      group: "bmad-solution",
      description: "PM: epics e stories granulares.",
      fields: [],
      action: { type: "command", base: "/bmad-create-epics-and-stories" },
      advance: {},
    },
    {
      id: "bmad-readiness",
      label: "Readiness",
      status: "bmad_readiness",
      group: "bmad-solution",
      description: "Architect: gate de prontidão (PASS/CONCERNS/FAIL).",
      fields: [],
      action: { type: "command", base: "/bmad-check-implementation-readiness" },
      advance: {},
    },
    {
      id: "bmad-dev",
      label: "Dev story",
      status: "bmad_dev",
      group: "bmad-impl",
      wipLimit: 1,
      description: "Developer: implementa uma story por vez.",
      fields: [],
      action: { type: "command", base: "/bmad-dev-story" },
      advance: {},
    },
    {
      id: "bmad-review",
      label: "Code review",
      status: "bmad_review",
      group: "bmad-impl",
      description: "Developer: code review + feedback de PR.",
      fields: [],
      action: { type: "command", base: "/bmad-code-review" },
      advance: {},
    },
  ],
};

// ---------------------------------------------------------------------------
// Built-in presets — applied to a workspace via the preset picker.
// ---------------------------------------------------------------------------

export type FlowPreset = { meta: FlowMeta; schema: WorkbenchSchema };

export const BUILTIN_PRESETS: Record<string, FlowPreset> = {
  "dev-workflow": {
    meta: {
      id: "dev-workflow",
      label: "dev-workflow",
      description: "Pipeline PRD → TechSpec → Tasks → Run → Review → QA → Security → Commit → PR.",
      preset: "dev-workflow",
      analyzeCommand: "/dw-analyze-project",
      analyzeMarker: ".dw/rules/index.md",
      suggestCommand: "/dw-opportunities",
    },
    schema: DEFAULT_WORKBENCH_SCHEMA,
  },
  "spec-kit": {
    meta: {
      id: "spec-kit",
      label: "GitHub spec-kit",
      description: "Pipeline /speckit.*: constitution → specify → clarify → plan → tasks → analyze → implement.",
      preset: "spec-kit",
    },
    schema: PRESET_SPEC_KIT,
  },
  openspec: {
    meta: {
      id: "openspec",
      label: "OpenSpec",
      description: "Change proposals /opsx:*: propose → apply → verify → sync → archive.",
      preset: "openspec",
    },
    schema: PRESET_OPENSPEC,
  },
  bmad: {
    meta: {
      id: "bmad",
      label: "BMAD-METHOD",
      description: "Personas: brief → PRD → UX → architecture → stories → readiness → dev → review.",
      preset: "bmad",
    },
    schema: PRESET_BMAD,
  },
};

export const BUILTIN_PRESET_LIST: FlowPreset[] = Object.values(BUILTIN_PRESETS);

// ---------------------------------------------------------------------------
// Parsing + loading
// ---------------------------------------------------------------------------

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

const FIELD_TYPES: WorkbenchFieldType[] = [
  "text",
  "textarea",
  "select",
  "multiselect",
  "checklist",
  "projects",
  "attachments",
];

/** Light validation of an intake field (the index.json intake block). */
function normalizeIntakeField(raw: unknown): WorkbenchField | null {
  if (!isRecord(raw) || typeof raw.key !== "string" || !raw.key.trim()) return null;
  const type = FIELD_TYPES.includes(raw.type as WorkbenchFieldType)
    ? (raw.type as WorkbenchFieldType)
    : "text";
  const binding = raw.binding === "title" || raw.binding === "body" ? raw.binding : undefined;
  return {
    key: raw.key,
    label: typeof raw.label === "string" && raw.label.trim() ? raw.label : raw.key,
    type,
    required: raw.required === true,
    placeholder: typeof raw.placeholder === "string" ? raw.placeholder : undefined,
    binding,
  };
}

/** Validate a `.dw/flows/index.json` payload; returns null on anything unusable. */
export function parseFlowIndex(raw: string | unknown): FlowIndex | null {
  let value: unknown = raw;
  if (typeof raw === "string") {
    try {
      value = JSON.parse(raw);
    } catch {
      return null;
    }
  }
  if (!isRecord(value) || !Array.isArray(value.flows)) return null;

  const flows: FlowMeta[] = [];
  const seen = new Set<string>();
  for (const entry of value.flows) {
    if (!isRecord(entry)) continue;
    const id = typeof entry.id === "string" ? entry.id.trim() : "";
    if (!id || seen.has(id)) continue;
    seen.add(id);
    flows.push({
      id,
      label: typeof entry.label === "string" && entry.label.trim() ? entry.label : id,
      description: typeof entry.description === "string" ? entry.description : undefined,
      preset: typeof entry.preset === "string" ? entry.preset : undefined,
      analyzeCommand: typeof entry.analyzeCommand === "string" ? entry.analyzeCommand : undefined,
      analyzeMarker: typeof entry.analyzeMarker === "string" ? entry.analyzeMarker : undefined,
      suggestCommand: typeof entry.suggestCommand === "string" ? entry.suggestCommand : undefined,
    });
  }
  if (flows.length === 0) return null;

  let intake: { fields: WorkbenchField[] } | undefined;
  if (isRecord(value.intake) && Array.isArray(value.intake.fields)) {
    const fields = value.intake.fields
      .map(normalizeIntakeField)
      .filter((field): field is WorkbenchField => field !== null);
    if (fields.length > 0) intake = { fields };
  }

  return {
    flows,
    default: typeof value.default === "string" ? value.default : undefined,
    intake,
  };
}

type ArtifactResult = { ok: true; value: string } | { ok: false; error: string };

// ---------------------------------------------------------------------------
// Assisted routing — recommend the most coherent flow for a card.
// ---------------------------------------------------------------------------

const STOP_WORDS = new Set([
  "the",
  "and",
  "for",
  "with",
  "que",
  "com",
  "para",
  "dos",
  "das",
  "uma",
  "fluxo",
  "flow",
  "card",
  "este",
  "esse",
  "esta",
]);

function tokenize(text: string): Set<string> {
  const tokens = new Set<string>();
  for (const raw of text.toLowerCase().split(/[^a-z0-9]+/)) {
    if (raw.length >= 4 && !STOP_WORDS.has(raw)) tokens.add(raw);
  }
  return tokens;
}

/**
 * Recommend the flow whose vocabulary (label + description + phase labels) best
 * overlaps the card's title/body. Deterministic — no agent round-trip. Returns
 * null when nothing matches (caller falls back to the first flow).
 */
export function recommendFlow(
  card: { title?: string | null; body?: string | null },
  registry: FlowRegistry,
): { flowId: string; score: number; matched: string[] } | null {
  const cardTokens = tokenize(`${card.title ?? ""} ${card.body ?? ""}`);
  if (cardTokens.size === 0) return null;

  let best: { flowId: string; score: number; matched: string[] } | null = null;
  for (const meta of registry.flows) {
    const schema = registry.schemas[meta.id];
    const vocab = tokenize(
      [meta.label, meta.description ?? "", ...(schema?.phases ?? []).map((phase) => phase.label)].join(
        " ",
      ),
    );
    const matched = [...vocab].filter((token) => cardTokens.has(token));
    if (matched.length > 0 && (!best || matched.length > best.score)) {
      best = { flowId: meta.id, score: matched.length, matched };
    }
  }
  return best;
}

/** A single-flow registry (legacy `.dw/workbench.json` or the bundled default). */
export function singleFlowRegistry(
  schema: WorkbenchSchema,
  id = "dev-workflow",
  label = "dev-workflow",
): FlowRegistry {
  // Carry the bundled preset's meta (incl. analyze/suggest commands) when it matches.
  const presetMeta = BUILTIN_PRESETS[id]?.meta;
  return {
    flows: [
      presetMeta
        ? { ...presetMeta, id, label }
        : { id, label, preset: undefined },
    ],
    schemas: { [id]: schema },
    defaultFlowId: id,
    intakeFields: schema.newCard?.fields ?? DEFAULT_INTAKE_FIELDS,
  };
}

/**
 * Load the flow registry for a project. Reads `.dw/flows/index.json`; for each
 * listed flow reads `.dw/flows/<id>.json` (falling back to a matching built-in
 * preset when the file is missing). With no index, falls back to the legacy
 * single `.dw/workbench.json` (or the bundled default) as one "dev-workflow" flow.
 */
export async function loadFlowRegistry(
  readArtifact: (relativePath: string) => Promise<ArtifactResult>,
): Promise<FlowRegistry> {
  const indexResult = await readArtifact("flows/index.json");
  const index = indexResult.ok ? parseFlowIndex(indexResult.value) : null;

  if (!index) {
    const legacy = await readArtifact("workbench.json");
    const schema = legacy.ok ? parseWorkbenchSchema(legacy.value).schema : DEFAULT_WORKBENCH_SCHEMA;
    return singleFlowRegistry(schema);
  }

  const schemas: Record<string, WorkbenchSchema> = {};
  const flows: FlowMeta[] = [];
  for (const meta of index.flows) {
    const fileResult = await readArtifact(`flows/${meta.id}.json`);
    let schema: WorkbenchSchema | undefined;
    if (fileResult.ok) {
      schema = parseWorkbenchSchema(fileResult.value).schema;
    } else if (meta.preset && BUILTIN_PRESETS[meta.preset]) {
      schema = BUILTIN_PRESETS[meta.preset].schema;
    } else if (BUILTIN_PRESETS[meta.id]) {
      schema = BUILTIN_PRESETS[meta.id].schema;
    }
    if (!schema) continue;
    schemas[meta.id] = schema;
    // Backfill analyze/suggest metadata from the bundled preset when the index omits it.
    const presetMeta = BUILTIN_PRESETS[meta.preset ?? meta.id]?.meta;
    flows.push({
      ...meta,
      analyzeCommand: meta.analyzeCommand ?? presetMeta?.analyzeCommand,
      analyzeMarker: meta.analyzeMarker ?? presetMeta?.analyzeMarker,
      suggestCommand: meta.suggestCommand ?? presetMeta?.suggestCommand,
    });
  }

  if (flows.length === 0) return singleFlowRegistry(DEFAULT_WORKBENCH_SCHEMA);

  const defaultFlowId = index.default && schemas[index.default] ? index.default : flows[0].id;
  return {
    flows,
    schemas,
    defaultFlowId,
    intakeFields: index.intake?.fields ?? DEFAULT_INTAKE_FIELDS,
  };
}
