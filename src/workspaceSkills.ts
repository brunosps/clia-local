import type { WorkspaceSkill } from "./types";

export type SkillSlashCommand =
  | { ok: true; skill: WorkspaceSkill; request: string }
  | { ok: false; error: string };

export type SkillSuggestionGroup = {
  scopeLabel: string;
  frameworkLabel: string;
  label: string;
  skills: WorkspaceSkill[];
};

const DEFAULT_SKILL_REQUEST =
  "Execute esta skill usando o contexto atual do workspace/projeto. Se faltar informação essencial, pergunte antes de continuar.";

export function isSkillSlashCommand(value: string) {
  const trimmed = value.trimStart();
  return /^\/[^\s/]+(?:\s|$)/.test(trimmed);
}

export function resolveSkillSlashCommand(
  value: string,
  skills: WorkspaceSkill[],
): SkillSlashCommand | null {
  const parsed = parseSkillCommand(value);
  if (!parsed) return null;

  if (parsed.legacy && !parsed.name) {
    return { ok: false, error: "Informe a skill após /skill." };
  }

  if (!parsed.name) return null;

  const skill = matchSkill(parsed.name, skills);
  if (!skill) {
    return parsed.legacy
      ? { ok: false, error: `Skill não encontrada no workspace: ${parsed.name}` }
      : null;
  }

  const request = parsed.request.trim() || DEFAULT_SKILL_REQUEST;

  return { ok: true, skill, request };
}

export function workspaceSkillFilePath(workspaceRoot: string, skill: WorkspaceSkill) {
  if (skill.path) {
    if (isAbsolutePath(skill.path)) return skill.path;
    return joinPath(workspaceRoot, skill.path);
  }
  return joinPath(workspaceRoot, ".dw/gui/skills", skill.name, "SKILL.md");
}

export function composeSkillPrompt(skill: WorkspaceSkill, skillContent: string, request: string) {
  return [
    `Use a skill de workspace "${skill.name}" abaixo para responder/executar o pedido.`,
    "Não procure essa skill no filesystem; o ADE já resolveu e injetou o conteúdo correto.",
    "",
    "<workspace_skill>",
    skillContent.trim(),
    "</workspace_skill>",
    "",
    "Pedido do usuário:",
    request.trim(),
  ].join("\n");
}

export function skillAutocompleteQuery(value: string): string | null {
  const trimmed = value.trimStart();
  if (!trimmed.startsWith("/")) return null;
  const firstLine = trimmed.split(/\r?\n/, 1)[0] ?? "";
  if (/^\/skill(?:\s|$)/.test(firstLine)) {
    const afterCommand = firstLine.slice("/skill".length);
    const skillPart = afterCommand.trimStart();
    if (skillPart.includes(" ")) return null;
    return skillPart;
  }
  const query = firstLine.slice(1);
  if (query.includes(" ")) return null;
  return query;
}

export function applySkillAutocomplete(value: string, skillName: string) {
  const leading = value.match(/^\s*/)?.[0] ?? "";
  const trimmed = value.trimStart();
  const lineBreak = trimmed.search(/\r?\n/);
  const suffix = lineBreak >= 0 ? trimmed.slice(lineBreak) : "";
  return `${leading}/${skillName} ${suffix}`;
}

export function filterSkillSuggestions(skills: WorkspaceSkill[], query: string | null) {
  if (query == null) return [];
  const normalized = normalizeSearchText(query);
  return orderSkills(skills)
    .filter((skill) => skillMatchesQuery(skill, normalized))
    .slice(0, 16);
}

export function skillGroupLabel(skill: WorkspaceSkill) {
  const explicit = skill.framework_label?.trim() || skill.group?.trim();
  if (explicit) return explicit;
  return skill.bundled ? "Bundled" : "Avulsas";
}

export function skillScopeLabel(skill: WorkspaceSkill) {
  const explicit = skill.scope_label?.trim();
  if (explicit) return explicit;
  if (skill.scope === "project") return "Projeto";
  if (skill.scope === "home") return "Home";
  return "Workspace";
}

export function groupSkillSuggestions(skills: WorkspaceSkill[]): SkillSuggestionGroup[] {
  const groups = new Map<
    string,
    { scopeLabel: string; frameworkLabel: string; skills: WorkspaceSkill[] }
  >();
  for (const skill of orderSkills(skills)) {
    const scopeLabel = skillScopeLabel(skill);
    const frameworkLabel = skillGroupLabel(skill);
    const key = `${scopeLabel}\u0000${frameworkLabel}`;
    const current = groups.get(key) ?? { scopeLabel, frameworkLabel, skills: [] };
    current.skills.push(skill);
    groups.set(key, current);
  }
  return [...groups.values()]
    .sort(
      (left, right) =>
        scopeRank(left.scopeLabel) - scopeRank(right.scopeLabel) ||
        skillGroupRank(left.frameworkLabel) - skillGroupRank(right.frameworkLabel) ||
        left.frameworkLabel.toLowerCase().localeCompare(right.frameworkLabel.toLowerCase()),
    )
    .map((group) => ({
      scopeLabel: group.scopeLabel,
      frameworkLabel: group.frameworkLabel,
      label: `${group.scopeLabel} / ${group.frameworkLabel}`,
      skills: [...group.skills].sort((left, right) => left.name.localeCompare(right.name)),
    }));
}

function joinPath(...parts: string[]) {
  return parts
    .filter(Boolean)
    .map((part, index) =>
      index === 0 ? part.replace(/\/+$/g, "") : part.replace(/^\/+|\/+$/g, ""),
    )
    .filter(Boolean)
    .join("/");
}

function isAbsolutePath(path: string) {
  return path.startsWith("/") || /^[A-Za-z]:[\\/]/.test(path);
}

function parseSkillCommand(
  value: string,
): { legacy: boolean; name: string; request: string } | null {
  const trimmed = value.trim();
  if (!trimmed.startsWith("/")) return null;

  const legacyMatch = trimmed.match(/^\/skill(?:\s+([\s\S]*))?$/);
  if (legacyMatch) {
    const rest = (legacyMatch[1] ?? "").trimStart();
    const nameMatch = rest.match(/^(\S+)(?:\s+([\s\S]*))?$/);
    return {
      legacy: true,
      name: (nameMatch?.[1] ?? "").replace(/^\$/, "").trim(),
      request: nameMatch?.[2] ?? "",
    };
  }

  const directMatch = trimmed.match(/^\/(\S+)(?:\s+([\s\S]*))?$/);
  if (!directMatch) return null;
  return {
    legacy: false,
    name: directMatch[1].replace(/^\$/, "").trim(),
    request: directMatch[2] ?? "",
  };
}

function matchSkill(name: string, skills: WorkspaceSkill[]) {
  const requestedName = name.toLowerCase();
  const ordered = orderSkills(skills);
  const exact = ordered.find((item) => item.name.toLowerCase() === requestedName);
  if (exact) return exact;

  const prefixMatches = ordered.filter((item) => item.name.toLowerCase().startsWith(requestedName));
  if (prefixMatches.length === 1) return prefixMatches[0];

  const containsMatches = ordered.filter((item) => item.name.toLowerCase().includes(requestedName));
  return containsMatches.length === 1 ? containsMatches[0] : null;
}

function skillMatchesQuery(skill: WorkspaceSkill, query: string) {
  if (!query) return true;
  const name = normalizeSearchText(skill.name);
  if (name.includes(query)) return true;

  const framework = normalizeSearchText(skill.framework_label ?? skill.group ?? "");
  if (query.length >= 2 && framework.includes(query)) return true;

  const scope = normalizeSearchText(skill.scope_label ?? "");
  if (query.length >= 3 && scope.includes(query)) return true;

  const description = normalizeSearchText(skill.description ?? "");
  return query.length >= 3 && description.includes(query);
}

function normalizeSearchText(value: string) {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, " ")
    .trim()
    .replace(/\s+/g, " ");
}

function skillGroupRank(label: string) {
  const normalized = label.toLowerCase();
  if (normalized !== "avulsas" && normalized !== "bundled") return 0;
  if (normalized === "bundled") return 1;
  if (normalized === "avulsas") return 2;
  return 1;
}

function orderSkills(skills: WorkspaceSkill[]) {
  return [...skills].sort(
    (left, right) =>
      (left.priority ?? scopePriority(left)) - (right.priority ?? scopePriority(right)) ||
      skillScopeLabel(left).localeCompare(skillScopeLabel(right)) ||
      skillGroupLabel(left).localeCompare(skillGroupLabel(right)) ||
      left.name.localeCompare(right.name),
  );
}

function scopePriority(skill: WorkspaceSkill) {
  if (skill.scope === "project") return 0;
  if (skill.scope === "workspace") return 1;
  if (skill.scope === "home") return 2;
  return 3;
}

function scopeRank(scopeLabel: string) {
  if (scopeLabel.startsWith("Projeto")) return 0;
  if (scopeLabel === "Workspace") return 1;
  if (scopeLabel.startsWith("Home")) return 2;
  return 3;
}
