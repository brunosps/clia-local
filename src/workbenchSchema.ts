import type { RequirementCard } from "./types";
import { quoteCommandArg, requirementPrdSlug } from "./workflow";

// ---------------------------------------------------------------------------
// Schema types — declarative workbench pipeline (.dw/workbench.json)
// ---------------------------------------------------------------------------

export type WorkbenchFieldType =
  | "text"
  | "textarea"
  | "select"
  | "multiselect"
  | "checklist"
  | "projects"
  | "attachments";

/** Binds a text/textarea field to a card-level property instead of the stage form. */
export type WorkbenchFieldBinding = "title" | "body";

export type WorkbenchFieldOption = {
  value: string;
  label: string;
  description?: string;
};

export type WorkbenchField = {
  key: string;
  label: string;
  type: WorkbenchFieldType;
  required?: boolean;
  placeholder?: string;
  options?: WorkbenchFieldOption[];
  help?: string;
  binding?: WorkbenchFieldBinding;
};

/** Atomic condition over the stage form. Multiple conditions are AND-ed. */
export type WorkbenchCondition = {
  field: string;
  equals?: string;
  notEquals?: string;
  nonEmpty?: boolean;
  empty?: boolean;
};

export type WorkbenchFlag = {
  flag?: string; // literal flag, e.g. "--coverage-only"
  flagTemplate?: string; // interpolated flag name, e.g. "--{{field.mode}}"
  style?: "bare" | "equals" | "space"; // default "bare"
  valueTemplate?: string; // value for equals/space styles
  when?: WorkbenchCondition[]; // AND
  whenAny?: WorkbenchCondition[]; // OR
};

export type WorkbenchPromptPart = {
  template: string;
  when?: WorkbenchCondition[]; // AND
  whenAny?: WorkbenchCondition[]; // OR
};

export type WorkbenchCommandVariant = {
  when?: WorkbenchCondition[]; // AND; omitted = always (fallback)
  whenAny?: WorkbenchCondition[]; // OR
  flags?: WorkbenchFlag[];
  promptParts?: WorkbenchPromptPart[];
};

export type WorkbenchCommandAction = {
  type: "command";
  base: string; // "/dw-plan"
  flags?: WorkbenchFlag[];
  promptParts?: WorkbenchPromptPart[]; // joined by " ", filtered, becomes ONE trailing quoted arg
  variants?: WorkbenchCommandVariant[]; // ordered; first matching wins
};

export type WorkbenchInterviewAction = {
  type: "interview";
  style: "horizons";
  minQuestions?: number; // default 5
  maxQuestions?: number; // default 9; 0 => uncapped
  fillsFields: string[]; // field keys the final JSON must map to
  instructions?: string;
};

export type WorkbenchSkillAction = {
  type: "skill";
  skill: string; // "/dw-secure-audit" or "$dw-ui-discipline"
  promptTemplate?: string;
};

export type WorkbenchNoneAction = { type: "none" };

export type WorkbenchAction =
  | WorkbenchCommandAction
  | WorkbenchInterviewAction
  | WorkbenchSkillAction
  | WorkbenchNoneAction;

/** Exit gate. All present conditions must pass (hard gate). Empty => no gate. */
export type WorkbenchAdvance = {
  requireFields?: string[];
  artifact?: string; // path template under .dw/, advance only when it exists
  expectJson?: string[]; // agent's structured final JSON must contain these keys
};

/** Stage archetype. Drives sensible defaults for artifacts/gates in the builder. */
export type StageKind =
  | "planning" // produces a required markdown doc (PRD/TechSpec/Tasks)
  | "execution" // produces a run log + evidence
  | "review" // checks; evidence optional
  | "delivery" // commit/PR
  | "approval" // human-only gate, no command/artifact
  | "status"; // just a board column (Backlog/Done)

/** An artifact (`.dw/` path template) a stage consumes. */
export type StageInput = {
  path: string;
  required?: boolean; // default true: missing blocks the stage in manual mode
};

/** The artifact a stage produces. */
export type StageOutput = {
  path: string; // `.dw/` path template
  policy: "none" | "optional" | "required"; // required => gates the exit
  capture?: boolean; // save the agent's final output to `path` automatically
};

export type WorkbenchPhase = {
  id: string;
  label: string;
  status: string;
  description: string;
  fields: WorkbenchField[];
  action: WorkbenchAction;
  advance?: WorkbenchAdvance;
  group?: string; // id of the WorkbenchGroup this phase belongs to
  wipLimit?: number; // max cards in progress for this phase (0/undefined = no limit)
  kind?: StageKind; // archetype; derived from action/group when omitted
  inputs?: StageInput[]; // artifacts consumed (handoff from earlier stages)
  output?: StageOutput; // artifact produced by this stage
};

/** A visual band that groups consecutive phases on the kanban (pastel-tinted). */
export type WorkbenchGroup = {
  id: string;
  label: string;
  color?: string; // base hex; falls back to the pastel palette by order
};

/** Simplified form shown when creating a new card (title/body/projects/attachments + custom). */
export type WorkbenchNewCard = {
  fields: WorkbenchField[];
};

export type WorkbenchSchema = {
  version: 1;
  newCard?: WorkbenchNewCard;
  groups?: WorkbenchGroup[];
  phases: WorkbenchPhase[];
};

// ---------------------------------------------------------------------------
// Interpolation — {{token}} with |join:<sep> and |default:<token-or-literal>
// ---------------------------------------------------------------------------

export type InterpolationScope = {
  card: Pick<RequirementCard, "title" | "slug" | "prd_slug" | "public_id" | "id">;
  form: Record<string, unknown>;
};

function resolveToken(token: string, scope: InterpolationScope): string | string[] {
  const trimmed = token.trim();
  if (trimmed === "card.title") return scope.card.title ?? "";
  if (trimmed === "card.slug") return scope.card.slug ?? "";
  if (trimmed === "prdSlug") return requirementPrdSlug(scope.card);
  if (trimmed === "cardBase") return cardArtifactBase(scope.card);
  if (trimmed.startsWith("field.")) {
    const value = scope.form[trimmed.slice("field.".length)];
    if (typeof value === "string") return value;
    if (Array.isArray(value)) return value.filter((item): item is string => typeof item === "string");
    return "";
  }
  return "";
}

function asText(value: string | string[]): string {
  return Array.isArray(value) ? value.join(" ") : value;
}

function isTokenRef(arg: string): boolean {
  const trimmed = arg.trim();
  return (
    trimmed === "card.title" ||
    trimmed === "card.slug" ||
    trimmed === "prdSlug" ||
    trimmed === "cardBase" ||
    trimmed.startsWith("field.")
  );
}

function applyFilter(value: string | string[], filter: string, scope: InterpolationScope): string | string[] {
  const [name, ...rest] = filter.split(":");
  const arg = rest.join(":");
  if (name === "join") {
    const sep = arg || ",";
    return Array.isArray(value) ? value.join(sep) : value;
  }
  if (name === "default") {
    const isEmpty = Array.isArray(value) ? value.length === 0 : value.trim() === "";
    if (!isEmpty) return value;
    // A token-shaped arg resolves through resolveToken (empty result keeps the
    // chain going, so `slug|default:scope|default:card.title` falls through).
    // A non-token arg is used as a literal fallback.
    return isTokenRef(arg) ? resolveToken(arg, scope) : arg;
  }
  return value;
}

export function phaseInterpolate(template: string, scope: InterpolationScope): string {
  return template.replace(/\{\{([^}]+)\}\}/g, (_match, expr: string) => {
    const [tokenName, ...filters] = expr.split("|").map((part) => part.trim());
    let value: string | string[] = resolveToken(tokenName, scope);
    for (const filter of filters) {
      value = applyFilter(value, filter, scope);
    }
    return asText(value);
  });
}

export function cardArtifactBase(card: Pick<RequirementCard, "public_id" | "id">): string {
  const id = card.public_id?.trim() || String(card.id);
  return `workbench/cards/${id}`;
}

// ---------------------------------------------------------------------------
// Condition evaluation
// ---------------------------------------------------------------------------

function fieldIsEmpty(value: unknown): boolean {
  if (value === undefined || value === null) return true;
  if (Array.isArray(value)) return value.length === 0;
  if (typeof value === "string") return value.trim() === "";
  return false;
}

function matchOne(form: Record<string, unknown>, condition: WorkbenchCondition): boolean {
  const value = form[condition.field];
  const text = typeof value === "string" ? value : "";
  if (condition.equals !== undefined && text !== condition.equals) return false;
  if (condition.notEquals !== undefined && text === condition.notEquals) return false;
  if (condition.nonEmpty && fieldIsEmpty(value)) return false;
  if (condition.empty && !fieldIsEmpty(value)) return false;
  return true;
}

function matchConditions(
  form: Record<string, unknown>,
  when?: WorkbenchCondition[],
  whenAny?: WorkbenchCondition[],
): boolean {
  if (when && !when.every((condition) => matchOne(form, condition))) return false;
  if (whenAny && whenAny.length > 0 && !whenAny.some((condition) => matchOne(form, condition))) {
    return false;
  }
  return true;
}

// ---------------------------------------------------------------------------
// Command building — replaces commandForRequirementStage
// ---------------------------------------------------------------------------

function renderFlag(flag: WorkbenchFlag, scope: InterpolationScope): string {
  const name = flag.flagTemplate ? phaseInterpolate(flag.flagTemplate, scope) : flag.flag ?? "";
  if (!name) return "";
  const style = flag.style ?? "bare";
  if (style === "bare") return name;
  const value = flag.valueTemplate ? phaseInterpolate(flag.valueTemplate, scope) : "";
  if (!value) return "";
  return style === "equals" ? `${name}=${value}` : `${name} ${value}`;
}

function renderPromptArg(parts: WorkbenchPromptPart[] | undefined, scope: InterpolationScope): string {
  if (!parts || parts.length === 0) return "";
  const rendered = parts
    .filter((part) => matchConditions(scope.form, part.when, part.whenAny))
    .map((part) => phaseInterpolate(part.template, scope).trim())
    .filter(Boolean)
    .join(" ");
  return rendered ? quoteCommandArg(rendered) : "";
}

export function buildPhaseCommand(
  phase: WorkbenchPhase,
  card: InterpolationScope["card"],
  form: Record<string, unknown> = {},
): string {
  const action = phase.action;
  if (action.type === "none" || action.type === "interview") return "";
  const scope: InterpolationScope = { card, form };

  if (action.type === "skill") {
    const prompt = action.promptTemplate ? phaseInterpolate(action.promptTemplate, scope) : "";
    return [action.skill, prompt ? quoteCommandArg(prompt) : ""].filter(Boolean).join(" ");
  }

  // command
  let flags = action.flags;
  let promptParts = action.promptParts;
  if (action.variants && action.variants.length > 0) {
    const variant =
      action.variants.find((candidate) => matchConditions(form, candidate.when, candidate.whenAny)) ??
      undefined;
    if (variant) {
      flags = variant.flags ?? flags;
      promptParts = variant.promptParts ?? promptParts;
    }
  }

  const parts = [action.base];
  for (const flag of flags ?? []) {
    if (!matchConditions(form, flag.when, flag.whenAny)) continue;
    const rendered = renderFlag(flag, scope);
    if (rendered) parts.push(rendered);
  }
  const promptArg = renderPromptArg(promptParts, scope);
  if (promptArg) parts.push(promptArg);
  return parts.join(" ");
}

// ---------------------------------------------------------------------------
// Phase lookup helpers — replace requirementStages / stageForRequirement*
// ---------------------------------------------------------------------------

export function firstPhaseId(schema: WorkbenchSchema): string {
  return schema.phases[0]?.id ?? "backlog";
}

export function phaseForStatus(phases: WorkbenchPhase[], status: string): WorkbenchPhase {
  return phases.find((phase) => phase.status === status) ?? phases[0];
}

export function phaseForCard(
  phases: WorkbenchPhase[],
  card: Pick<RequirementCard, "status" | "archived_from_status">,
): WorkbenchPhase {
  if (card.status === "archived" && card.archived_from_status) {
    return phaseForStatus(phases, card.archived_from_status);
  }
  return phaseForStatus(phases, card.status);
}

export function phaseById(phases: WorkbenchPhase[], id: string): WorkbenchPhase | undefined {
  return phases.find((phase) => phase.id === id);
}

export function firstPhase(schema: WorkbenchSchema): WorkbenchPhase {
  return schema.phases[0];
}

/** Default pastel bases (hex) assigned to groups by order when no color is set. */
export const PASTEL_PALETTE = ["#6ea8fe", "#5fd0a8", "#b99cff", "#f2c879", "#f29db4", "#7fd1e8"];

export type PhaseGroupBand = {
  group: WorkbenchGroup;
  color: string;
  phases: WorkbenchPhase[];
};

/**
 * Order phases into group bands. Groups follow `schema.groups`; phases without a
 * matching group fall into a trailing "_ungrouped" band. With no groups defined,
 * returns a single unlabeled band containing every phase (flat kanban).
 */
export function groupedPhases(schema: WorkbenchSchema): PhaseGroupBand[] {
  const groups = schema.groups ?? [];
  if (groups.length === 0) {
    return [{ group: { id: "_ungrouped", label: "" }, color: PASTEL_PALETTE[0], phases: schema.phases }];
  }

  const bands: PhaseGroupBand[] = [];
  const used = new Set<string>();
  groups.forEach((group, index) => {
    const phases = schema.phases.filter((phase) => phase.group === group.id);
    phases.forEach((phase) => used.add(phase.id));
    bands.push({
      group,
      color: group.color || PASTEL_PALETTE[index % PASTEL_PALETTE.length],
      phases,
    });
  });

  const orphans = schema.phases.filter((phase) => !used.has(phase.id));
  if (orphans.length > 0) {
    bands.push({ group: { id: "_ungrouped", label: "Outros" }, color: PASTEL_PALETTE[0], phases: orphans });
  }
  return bands.filter((band) => band.phases.length > 0);
}

/** A field that maps to card-level data (title/body/projects/attachments) rather than stage form. */
export function isCardLevelField(field: WorkbenchField): boolean {
  return (
    field.binding === "title" ||
    field.binding === "body" ||
    field.type === "projects" ||
    field.type === "attachments"
  );
}

export type ClassifiedCardFields = {
  titleField?: WorkbenchField;
  bodyField?: WorkbenchField;
  projectsField?: WorkbenchField;
  attachmentsField?: WorkbenchField;
  customFields: WorkbenchField[];
};

/** Split a field list into the built-in card-level slots + remaining custom fields. */
export function classifyCardFields(fields: WorkbenchField[]): ClassifiedCardFields {
  const result: ClassifiedCardFields = { customFields: [] };
  for (const field of fields) {
    if (field.binding === "title") result.titleField = field;
    else if (field.binding === "body") result.bodyField = field;
    else if (field.type === "projects") result.projectsField = field;
    else if (field.type === "attachments") result.attachmentsField = field;
    else result.customFields.push(field);
  }
  return result;
}

/** Fields for the "new card" form: the dedicated newCard block, or the first phase as fallback. */
export function newCardFields(schema: WorkbenchSchema): WorkbenchField[] {
  return schema.newCard?.fields ?? schema.phases[0]?.fields ?? [];
}

export function nextPhase(phases: WorkbenchPhase[], id: string): WorkbenchPhase | undefined {
  const index = phases.findIndex((phase) => phase.id === id);
  if (index === -1) return undefined;
  return phases[index + 1];
}

// ---------------------------------------------------------------------------
// Advance gate — pure; artifact existence precomputed by the caller
// ---------------------------------------------------------------------------

export type AdvanceContext = {
  form: Record<string, unknown>;
  artifactExists?: boolean;
  outputExists?: boolean; // does phase.output.path exist on disk?
  agentJson?: Record<string, unknown> | null;
  approved?: boolean; // human confirmed an `approval` stage
  override?: boolean; // forcing past the gate (with a recorded reason)
};

/** Stage archetype, derived from `action`/`group` when not declared. */
export function stageKind(phase: WorkbenchPhase): StageKind {
  if (phase.kind) return phase.kind;
  if (phase.action.type === "none") return "status";
  switch (phase.group) {
    case "planejamento":
      return "planning";
    case "entrega":
      return "delivery";
    case "concluido":
      return "status";
    default:
      return "execution";
  }
}

/** Default output policy for a stage archetype (used by the builder + migration). */
export function stageDefaultOutputPolicy(kind: StageKind): StageOutput["policy"] {
  switch (kind) {
    case "planning":
      return "required";
    case "execution":
    case "review":
      return "optional";
    default:
      return "none";
  }
}

export function canAdvancePhase(
  phase: WorkbenchPhase,
  ctx: AdvanceContext,
): { ok: boolean; reasons: string[] } {
  if (ctx.override) return { ok: true, reasons: [] };
  const reasons: string[] = [];
  const advance = phase.advance;

  for (const key of advance?.requireFields ?? []) {
    if (fieldIsEmpty(ctx.form[key])) {
      const label = phase.fields.find((field) => field.key === key)?.label ?? key;
      reasons.push(`Campo obrigatório "${label}" não preenchido.`);
    }
  }
  if (advance?.artifact && !ctx.artifactExists) {
    reasons.push(`Artefato esperado ausente: .dw/${advance.artifact}`);
  }
  for (const key of advance?.expectJson ?? []) {
    if (!ctx.agentJson || fieldIsEmpty(ctx.agentJson[key])) {
      reasons.push(`Retorno do agente sem a chave esperada "${key}".`);
    }
  }
  if (phase.output?.policy === "required" && !ctx.outputExists) {
    reasons.push(`Documento de saída ausente: .dw/${phase.output.path}`);
  }
  if (stageKind(phase) === "approval" && !ctx.approved) {
    reasons.push("Aprovação humana pendente nesta etapa.");
  }
  return { ok: reasons.length === 0, reasons };
}

/** Resolve the artifact path template for a phase (for the caller's existence check). */
export function advanceArtifactPath(
  phase: WorkbenchPhase,
  card: InterpolationScope["card"],
  form: Record<string, unknown> = {},
): string | null {
  if (!phase.advance?.artifact) return null;
  return phaseInterpolate(phase.advance.artifact, { card, form });
}

/** Resolved input artifacts a stage consumes (path + required flag). */
export function stageInputPaths(
  phase: WorkbenchPhase,
  card: InterpolationScope["card"],
  form: Record<string, unknown> = {},
): { path: string; required: boolean }[] {
  return (phase.inputs ?? []).map((input) => ({
    path: phaseInterpolate(input.path, { card, form }),
    required: input.required !== false,
  }));
}

/** Resolved output artifact path a stage produces, or null. */
export function stageOutputPath(
  phase: WorkbenchPhase,
  card: InterpolationScope["card"],
  form: Record<string, unknown> = {},
): string | null {
  if (!phase.output || phase.output.policy === "none") return null;
  return phaseInterpolate(phase.output.path, { card, form });
}

// ---------------------------------------------------------------------------
// Interview — generic prompt + output mapping
// ---------------------------------------------------------------------------

export const INTERVIEW_DEFAULT_MIN = 5;
export const INTERVIEW_DEFAULT_MAX = 9;

export function interviewActionFor(phase: WorkbenchPhase): WorkbenchInterviewAction | null {
  return phase.action.type === "interview" ? phase.action : null;
}

/** True when the phase delegates a runnable command/skill (vs interview/manual). */
export function phaseDelegatesCommand(phase: WorkbenchPhase): boolean {
  return phase.action.type === "command" || phase.action.type === "skill";
}

export function interviewPromptFor(
  phase: WorkbenchPhase,
  contextLines: string[] = [],
): string {
  const action = interviewActionFor(phase);
  if (!action) return "";
  const min = action.minQuestions ?? INTERVIEW_DEFAULT_MIN;
  const max = action.maxQuestions ?? INTERVIEW_DEFAULT_MAX;
  const capped = max > 0;

  const finalShape: Record<string, string> = { state: "final" };
  for (const key of action.fillsFields) {
    const field = phase.fields.find((item) => item.key === key);
    finalShape[key] = field?.type === "checklist" || field?.type === "multiselect" ? "[...]" : "...";
  }

  const lines = [
    `Você conduz uma entrevista curta para preencher os campos da fase "${phase.label}" de um card.`,
    "Responda SOMENTE com JSON válido, sem markdown e sem texto fora do JSON.",
    "",
    "Regras:",
    `- A entrevista deve ter no mínimo ${min} perguntas respondidas.`,
    capped
      ? `- A entrevista deve ter no máximo ${max} perguntas respondidas.`
      : "- Não há limite máximo de perguntas; finalize quando tiver informação suficiente.",
    "- Cada pergunta deve ter exatamente quatro opções de horizonte: H1, H2, H3, H4.",
    "- H1: conservador, mas com inovação prática.",
    "- H2: mais ousado.",
    "- H3: disruptivo.",
    "- H4: contraponto ou caminho contrário aos anteriores.",
    `- Use as respostas para preencher os campos: ${action.fillsFields.join(", ")}.`,
    action.instructions ? `- ${action.instructions}` : "",
    "",
    "Formato para pergunta:",
    JSON.stringify(
      {
        state: "question",
        question_number: 1,
        question: "...",
        options: { H1: "...", H2: "...", H3: "...", H4: "..." },
        running_summary: "...",
      },
      null,
      2,
    ),
    "",
    "Formato para final (preencha cada campo da fase):",
    JSON.stringify(finalShape, null, 2),
  ].filter(Boolean);

  return [...lines, ...(contextLines.length ? ["", ...contextLines] : [])].join("\n");
}

/** Map an agent's final JSON object onto the phase's declared field values. */
export function mapInterviewOutputToFields(
  phase: WorkbenchPhase,
  final: Record<string, unknown>,
): Record<string, string | string[]> {
  const action = interviewActionFor(phase);
  const keys = action?.fillsFields ?? phase.fields.map((field) => field.key);
  const result: Record<string, string | string[]> = {};
  for (const key of keys) {
    const value = final[key];
    const field = phase.fields.find((item) => item.key === key);
    const arrayLike = field?.type === "checklist" || field?.type === "multiselect";
    if (arrayLike) {
      result[key] = Array.isArray(value)
        ? value.filter((item): item is string => typeof item === "string" && item.trim() !== "")
        : [];
    } else {
      result[key] = typeof value === "string" ? value : "";
    }
  }
  return result;
}

// ---------------------------------------------------------------------------
// Parsing / validation — falls back to default on any problem
// ---------------------------------------------------------------------------

export type WorkbenchParseResult = {
  schema: WorkbenchSchema;
  warnings: string[];
  usedDefault: boolean;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

export function parseWorkbenchSchema(raw: string | unknown): WorkbenchParseResult {
  let value: unknown = raw;
  if (typeof raw === "string") {
    try {
      value = JSON.parse(raw);
    } catch {
      return { schema: DEFAULT_WORKBENCH_SCHEMA, warnings: ["JSON inválido."], usedDefault: true };
    }
  }
  if (!isRecord(value) || !Array.isArray(value.phases) || value.phases.length === 0) {
    return {
      schema: DEFAULT_WORKBENCH_SCHEMA,
      warnings: ["Schema sem fases válidas."],
      usedDefault: true,
    };
  }

  const warnings: string[] = [];
  const ids = new Set<string>();
  const phases: WorkbenchPhase[] = [];
  for (const rawPhase of value.phases) {
    if (!isRecord(rawPhase)) continue;
    const id = typeof rawPhase.id === "string" ? rawPhase.id.trim() : "";
    const status = typeof rawPhase.status === "string" ? rawPhase.status.trim() : id;
    if (!id || ids.has(id)) {
      warnings.push(`Fase inválida ou id duplicado: "${id || "(vazio)"}".`);
      continue;
    }
    ids.add(id);
    const fields = Array.isArray(rawPhase.fields)
      ? rawPhase.fields.filter(isRecord).map(normalizeField)
      : [];
    phases.push({
      id,
      label: typeof rawPhase.label === "string" ? rawPhase.label : id,
      status: status || id,
      description: typeof rawPhase.description === "string" ? rawPhase.description : "",
      fields,
      action: normalizeAction(rawPhase.action),
      advance: normalizeAdvance(rawPhase.advance, fields, warnings, id),
      group: typeof rawPhase.group === "string" ? rawPhase.group : undefined,
      wipLimit:
        typeof rawPhase.wipLimit === "number" && rawPhase.wipLimit > 0
          ? Math.floor(rawPhase.wipLimit)
          : undefined,
      kind: normalizeStageKind(rawPhase.kind),
      inputs: normalizeStageInputs(rawPhase.inputs),
      output: normalizeStageOutput(rawPhase.output),
    });
  }

  if (phases.length === 0) {
    return { schema: DEFAULT_WORKBENCH_SCHEMA, warnings: ["Nenhuma fase válida."], usedDefault: true };
  }

  let newCard: WorkbenchNewCard | undefined;
  if (isRecord(value.newCard) && Array.isArray(value.newCard.fields)) {
    newCard = { fields: value.newCard.fields.filter(isRecord).map(normalizeField) };
  }

  const groups = Array.isArray(value.groups)
    ? value.groups.filter(isRecord).flatMap((raw): WorkbenchGroup[] => {
        const id = typeof raw.id === "string" ? raw.id.trim() : "";
        if (!id) return [];
        return [
          {
            id,
            label: typeof raw.label === "string" ? raw.label : id,
            color: typeof raw.color === "string" ? raw.color : undefined,
          },
        ];
      })
    : undefined;

  return {
    schema: { version: 1, newCard, groups: groups?.length ? groups : undefined, phases },
    warnings,
    usedDefault: false,
  };
}

function normalizeField(raw: Record<string, unknown>): WorkbenchField {
  const type = raw.type;
  const fieldType: WorkbenchFieldType =
    type === "textarea" ||
    type === "select" ||
    type === "multiselect" ||
    type === "checklist" ||
    type === "projects" ||
    type === "attachments"
      ? type
      : "text";
  const binding: WorkbenchFieldBinding | undefined =
    raw.binding === "title" || raw.binding === "body" ? raw.binding : undefined;
  const options = Array.isArray(raw.options)
    ? raw.options.filter(isRecord).map((option) => ({
        value: typeof option.value === "string" ? option.value : "",
        label: typeof option.label === "string" ? option.label : String(option.value ?? ""),
        description: typeof option.description === "string" ? option.description : undefined,
      }))
    : undefined;
  return {
    key: typeof raw.key === "string" ? raw.key : "",
    label: typeof raw.label === "string" ? raw.label : (typeof raw.key === "string" ? raw.key : ""),
    type: fieldType,
    required: raw.required === true,
    placeholder: typeof raw.placeholder === "string" ? raw.placeholder : undefined,
    options,
    help: typeof raw.help === "string" ? raw.help : undefined,
    binding,
  };
}

function normalizeAction(raw: unknown): WorkbenchAction {
  if (!isRecord(raw)) return { type: "none" };
  if (raw.type === "interview") {
    return {
      type: "interview",
      style: "horizons",
      minQuestions: typeof raw.minQuestions === "number" ? raw.minQuestions : undefined,
      maxQuestions: typeof raw.maxQuestions === "number" ? raw.maxQuestions : undefined,
      fillsFields: Array.isArray(raw.fillsFields)
        ? raw.fillsFields.filter((item): item is string => typeof item === "string")
        : [],
      instructions: typeof raw.instructions === "string" ? raw.instructions : undefined,
    };
  }
  if (raw.type === "skill") {
    return {
      type: "skill",
      skill: typeof raw.skill === "string" ? raw.skill : "",
      promptTemplate: typeof raw.promptTemplate === "string" ? raw.promptTemplate : undefined,
    };
  }
  if (raw.type === "command") {
    return raw as unknown as WorkbenchCommandAction;
  }
  return { type: "none" };
}

function normalizeAdvance(
  raw: unknown,
  fields: WorkbenchField[],
  warnings: string[],
  phaseId: string,
): WorkbenchAdvance | undefined {
  if (!isRecord(raw)) return undefined;
  const fieldKeys = new Set(fields.map((field) => field.key));
  const requireFields = Array.isArray(raw.requireFields)
    ? raw.requireFields.filter((item): item is string => typeof item === "string")
    : undefined;
  for (const key of requireFields ?? []) {
    if (!fieldKeys.has(key)) {
      warnings.push(`Fase "${phaseId}": requireFields referencia campo inexistente "${key}".`);
    }
  }
  return {
    requireFields,
    artifact: typeof raw.artifact === "string" ? raw.artifact : undefined,
    expectJson: Array.isArray(raw.expectJson)
      ? raw.expectJson.filter((item): item is string => typeof item === "string")
      : undefined,
  };
}

const STAGE_KINDS: StageKind[] = [
  "planning",
  "execution",
  "review",
  "delivery",
  "approval",
  "status",
];

function normalizeStageKind(raw: unknown): StageKind | undefined {
  return STAGE_KINDS.includes(raw as StageKind) ? (raw as StageKind) : undefined;
}

function normalizeStageInputs(raw: unknown): StageInput[] | undefined {
  if (!Array.isArray(raw)) return undefined;
  const inputs = raw
    .filter(isRecord)
    .map((item): StageInput | null => {
      const path = typeof item.path === "string" ? item.path.trim() : "";
      if (!path) return null;
      return { path, required: item.required === false ? false : undefined };
    })
    .filter((item): item is StageInput => item !== null);
  return inputs.length ? inputs : undefined;
}

function normalizeStageOutput(raw: unknown): StageOutput | undefined {
  if (!isRecord(raw)) return undefined;
  const path = typeof raw.path === "string" ? raw.path.trim() : "";
  if (!path) return undefined;
  const policy =
    raw.policy === "optional" || raw.policy === "required" || raw.policy === "none"
      ? raw.policy
      : "optional";
  return { path, policy, capture: raw.capture === true };
}

// ---------------------------------------------------------------------------
// DEFAULT schema — reproduces the current hardcoded pipeline + new security phase
// ---------------------------------------------------------------------------

const BRAINSTORM_MODE_OPTIONS: WorkbenchFieldOption[] = [
  {
    value: "option-matrix",
    label: "Option matrix",
    description: "Matriz de opções conservadora, equilibrada e ousada.",
  },
  {
    value: "grill",
    label: "Grill",
    description: "Aperta vocabulário, nomes e termos que ainda estão confusos.",
  },
  {
    value: "prototype",
    label: "Prototype",
    description: "Explora rapidamente como a ideia deveria funcionar ou parecer.",
  },
  {
    value: "council",
    label: "Council",
    description: "Compara abordagens com visões diferentes antes de decidir.",
  },
  {
    value: "research",
    label: "Research",
    description: "Busca evidências externas quando a decisão depende de referências atuais.",
  },
  {
    value: "refactor-audit",
    label: "Refactor audit",
    description: "Investiga dívida técnica, complexidade e oportunidades de simplificação.",
  },
  {
    value: "onepager",
    label: "One-pager",
    description: "Gera um resumo durável da ideia quando ela já está convergindo.",
  },
];

export const DEFAULT_WORKBENCH_SCHEMA: WorkbenchSchema = {
  version: 1,
  newCard: {
    fields: [
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
    ],
  },
  groups: [
    { id: "planejamento", label: "Planejamento" },
    { id: "execucao", label: "Execução" },
    { id: "entrega", label: "Entrega" },
    { id: "concluido", label: "Concluído" },
  ],
  // Backlog/idea-capture is the shared Intake (flow-agnostic), not a flow phase.
  phases: [
    {
      id: "brainstorm",
      label: "Brainstorm",
      status: "brainstorming",
      group: "planejamento",
      description: "Explore the idea and risks before writing the PRD.",
      fields: [
        { key: "modes", label: "Modos", type: "multiselect", options: BRAINSTORM_MODE_OPTIONS },
        { key: "objective", label: "Objetivo", type: "textarea" },
      ],
      action: {
        type: "command",
        base: "/dw-brainstorm",
        flags: [
          {
            flag: "--mode",
            style: "equals",
            valueTemplate: "{{field.modes|join:+}}",
            when: [{ field: "modes", nonEmpty: true }],
          },
        ],
        promptParts: [
          {
            template: "{{field.objective|default:card.title}}",
            when: [
              { field: "context_path", empty: true },
              { field: "output_path", empty: true },
            ],
          },
          {
            template: "Leia .dw/{{field.context_path}} antes de responder.",
            when: [{ field: "context_path", nonEmpty: true }],
          },
          {
            template: "Grave o resultado em .dw/{{field.output_path}}.",
            when: [{ field: "output_path", nonEmpty: true }],
          },
          {
            template: "Objetivo: {{field.objective|default:card.title}}",
            whenAny: [
              { field: "context_path", nonEmpty: true },
              { field: "output_path", nonEmpty: true },
            ],
          },
        ],
      },
      advance: {},
      kind: "planning",
      output: { path: "{{cardBase}}/brainstorm.md", policy: "optional", capture: true },
    },
    {
      id: "plan",
      label: "Plan",
      status: "planned",
      group: "planejamento",
      description: "Generate PRD, TechSpec, and Tasks.",
      fields: [
        { key: "slug", label: "Slug ou ideia", type: "text" },
        { key: "acceptance", label: "Critérios", type: "text" },
      ],
      action: {
        type: "command",
        base: "/dw-plan",
        variants: [
          {
            when: [{ field: "brainstorm_output_path", nonEmpty: true }],
            promptParts: [
              {
                template:
                  "{{field.slug|default:field.scope|default:card.title}}. Se existir, use .dw/{{field.brainstorm_output_path}} como resultado do brainstorm.",
              },
            ],
          },
          {
            promptParts: [{ template: "{{field.slug|default:field.scope|default:card.title}}" }],
          },
        ],
      },
      advance: {},
      kind: "planning",
      inputs: [{ path: "{{cardBase}}/brainstorm.md", required: false }],
      // Sobe para "required" na Fase B (junto da captura + override UI).
      output: { path: "{{cardBase}}/plan.md", policy: "optional", capture: true },
    },
    {
      id: "run",
      label: "Run",
      status: "running",
      group: "execucao",
      wipLimit: 1,
      description: "Execute the approved work.",
      fields: [
        {
          key: "mode",
          label: "Modo",
          type: "select",
          options: [
            { value: "all", label: "Todas as tasks" },
            { value: "resume", label: "Resume" },
            { value: "task", label: "Task específica" },
          ],
        },
        { key: "task_id", label: "Task ID", type: "text" },
      ],
      action: {
        type: "command",
        base: "/dw-run",
        variants: [
          { when: [{ field: "mode", equals: "resume" }], flags: [{ flag: "--resume", style: "bare" }] },
          {
            when: [
              { field: "mode", equals: "task" },
              { field: "task_id", nonEmpty: true },
            ],
            promptParts: [{ template: "{{field.task_id}}" }],
          },
          { promptParts: [{ template: "{{prdSlug}}" }] },
        ],
      },
      advance: {},
      kind: "execution",
      inputs: [{ path: "{{cardBase}}/plan.md", required: false }],
      output: { path: "{{cardBase}}/run-log.md", policy: "optional", capture: true },
    },
    {
      id: "review",
      label: "Review",
      status: "reviewing",
      group: "execucao",
      wipLimit: 2,
      description: "Check coverage and code quality.",
      fields: [
        {
          key: "mode",
          label: "Modo",
          type: "select",
          options: [
            { value: "full", label: "Completo (L2+L3)" },
            { value: "coverage", label: "Só cobertura PRD" },
            { value: "code", label: "Só code review" },
          ],
        },
      ],
      action: {
        type: "command",
        base: "/dw-review",
        flags: [
          { flag: "--coverage-only", style: "bare", when: [{ field: "mode", equals: "coverage" }] },
          { flag: "--code-only", style: "bare", when: [{ field: "mode", equals: "code" }] },
        ],
        promptParts: [{ template: "{{prdSlug}}" }],
      },
      advance: {},
      kind: "review",
      output: { path: "{{cardBase}}/review.md", policy: "optional", capture: true },
    },
    {
      id: "qa",
      label: "QA",
      status: "qa",
      group: "execucao",
      wipLimit: 2,
      description: "Validate the behavior and capture evidence.",
      fields: [
        {
          key: "mode",
          label: "Modo",
          type: "select",
          options: [
            { value: "default", label: "Auto (UI/API)" },
            { value: "ui", label: "UI" },
            { value: "api", label: "API" },
            { value: "uat", label: "UAT" },
            { value: "ai", label: "AI/RAG" },
            { value: "fix", label: "Fix loop" },
          ],
        },
      ],
      action: {
        type: "command",
        base: "/dw-qa",
        flags: [
          { flag: "--fix", style: "bare", when: [{ field: "mode", equals: "fix" }] },
          {
            flagTemplate: "--{{field.mode}}",
            style: "bare",
            when: [
              { field: "mode", nonEmpty: true },
              { field: "mode", notEquals: "default" },
              { field: "mode", notEquals: "fix" },
            ],
          },
        ],
        promptParts: [{ template: "{{prdSlug}}" }],
      },
      advance: {},
      kind: "review",
      output: { path: "{{cardBase}}/qa-report.md", policy: "optional", capture: true },
    },
    {
      id: "security",
      label: "Security",
      status: "security",
      group: "execucao",
      wipLimit: 1,
      description: "Gate de segurança: Semgrep + gitleaks + Trivy + lockfile no diff.",
      fields: [],
      action: { type: "command", base: "/dw-secure-audit" },
      advance: {},
      kind: "review",
      output: { path: "{{cardBase}}/security-audit.md", policy: "optional", capture: true },
    },
    {
      id: "commit",
      label: "Commit",
      status: "ready_for_pr",
      group: "entrega",
      description: "Commit verified changes.",
      fields: [
        { key: "scope", label: "Escopo", type: "text" },
        { key: "message", label: "Mensagem", type: "text" },
      ],
      action: { type: "command", base: "/dw-commit" },
      advance: {},
      kind: "delivery",
    },
    {
      id: "local-pr",
      label: "Local PR",
      status: "local_pr",
      group: "entrega",
      description: "Generate the local PR package.",
      fields: [
        { key: "title", label: "Título", type: "text" },
        { key: "test_plan", label: "Test plan", type: "textarea" },
      ],
      action: { type: "command", base: "/dw-generate-pr" },
      advance: {},
      kind: "delivery",
    },
    {
      id: "done",
      label: "Done",
      status: "done",
      group: "concluido",
      description: "Work is complete.",
      fields: [],
      action: { type: "none" },
      kind: "status",
    },
  ],
};
