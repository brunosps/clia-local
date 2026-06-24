export type TabPreference = "queue" | "code" | "git" | "deploy" | "agents" | "settings";
export type SourceSideTabPreference = "explorer" | "search";
export type GitViewPreference = "local" | "commits";
export type GitDetailTabPreference = "commit" | "changes" | "tree";
export type ThemeMode = "clia" | "black";

export type SourceWorkspacePreference = {
  openPaths: string[];
  expandedPaths: string[];
  activePath: string | null;
  sideTab: SourceSideTabPreference;
  preview: boolean;
  showHistory: boolean;
};

export type GitWorkbenchPreference = {
  view: GitViewPreference;
  includeRemotes: boolean;
  includeTags: boolean;
  limit: number;
  detailTab: GitDetailTabPreference;
};

export const WORKSPACE_COLOR_PRESETS = [
  { label: "Azul", color: "#4f8cff" },
  { label: "Ciano", color: "#2aaec8" },
  { label: "Verde", color: "#43a047" },
  { label: "Âmbar", color: "#d18b22" },
  { label: "Rosa", color: "#d45d79" },
  { label: "Violeta", color: "#8b6fd6" },
  { label: "Índigo", color: "#5b6ee1" },
  { label: "Grafite", color: "#7b8794" },
] as const;

const VALID_TABS: TabPreference[] = ["queue", "code", "git", "deploy", "agents", "settings"];
const VALID_THEMES: ThemeMode[] = ["clia", "black"];
const VALID_GIT_VIEWS: GitViewPreference[] = ["local", "commits"];
const VALID_GIT_DETAIL_TABS: GitDetailTabPreference[] = ["commit", "changes", "tree"];

export function workspaceUiPreferenceKey(workspaceId: number, name: string): string {
  return `ui.workspace:${workspaceId}:${name}`;
}

export function projectUiPreferenceKey(projectId: number, name: string): string {
  return `ui.project:${projectId}:${name}`;
}

export function clampNumberPreference(
  value: unknown,
  fallback: number,
  bounds: { min: number; max: number },
): number {
  const parsed = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(parsed)) return fallback;
  return Math.max(bounds.min, Math.min(bounds.max, Math.round(parsed)));
}

export function parseTabPreference(value: string | null | undefined): TabPreference | null {
  if (value === "workbench") return "queue";
  if (value === "knowledge" || value === "skills") return "queue";
  if (value === "flows" || value === "machines") return "deploy";
  if (value === "cloud") return "settings";
  return VALID_TABS.includes(value as TabPreference) ? (value as TabPreference) : null;
}

export function parseThemePreference(value: string | null | undefined): ThemeMode {
  return VALID_THEMES.includes(value as ThemeMode) ? (value as ThemeMode) : "clia";
}

export function normalizeHexColorPreference(value: string | null | undefined): string | null {
  if (!value) return null;
  const trimmed = value.trim();
  const hex = trimmed.startsWith("#") ? trimmed : `#${trimmed}`;
  return /^#[0-9a-fA-F]{6}$/.test(hex) ? hex.toLowerCase() : null;
}

export function normalizeWorkspaceAccentColor(value: string | null | undefined): string | null {
  const normalized = normalizeHexColorPreference(value);
  return WORKSPACE_COLOR_PRESETS.some((preset) => preset.color === normalized) ? normalized : null;
}

export function workspaceAccentCssVariables(
  color: string | null | undefined,
): Record<`--${string}`, string> {
  const normalized = normalizeWorkspaceAccentColor(color);
  if (!normalized) return {};
  const r = Number.parseInt(normalized.slice(1, 3), 16);
  const g = Number.parseInt(normalized.slice(3, 5), 16);
  const b = Number.parseInt(normalized.slice(5, 7), 16);
  const onAccent = relativeLuminance(r, g, b) > 0.18 ? "#07111f" : "#f7fbff";
  return {
    "--workspace-accent": normalized,
    "--workspace-accent-rgb": `${r}, ${g}, ${b}`,
    "--workspace-accent-shell": `rgba(${r}, ${g}, ${b}, 0.06)`,
    "--workspace-accent-soft": `rgba(${r}, ${g}, ${b}, 0.14)`,
    "--workspace-accent-surface": `rgba(${r}, ${g}, ${b}, 0.18)`,
    "--workspace-accent-surface-strong": `rgba(${r}, ${g}, ${b}, 0.26)`,
    "--workspace-accent-muted": `rgba(${r}, ${g}, ${b}, 0.08)`,
    "--workspace-accent-border": `rgba(${r}, ${g}, ${b}, 0.5)`,
    "--workspace-accent-ring": `rgba(${r}, ${g}, ${b}, 0.78)`,
    "--workspace-accent-text": `color-mix(in srgb, ${normalized} 76%, #ffffff)`,
    "--workspace-accent-panel": `color-mix(in srgb, #151923 88%, ${normalized})`,
    "--workspace-accent-panel-strong": `color-mix(in srgb, #202633 76%, ${normalized})`,
    "--workspace-accent-button": `color-mix(in srgb, ${normalized} 42%, #d8e8ff)`,
    "--workspace-accent-on": onAccent,
  };
}

function relativeLuminance(r: number, g: number, b: number): number {
  const [red, green, blue] = [r, g, b].map((channel) => {
    const value = channel / 255;
    return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
}

function parseJsonRecord(value: string | null | undefined): Record<string, unknown> | null {
  if (!value) return null;
  try {
    const parsed = JSON.parse(value);
    if (typeof parsed === "object" && parsed && !Array.isArray(parsed)) {
      return parsed as Record<string, unknown>;
    }
  } catch {
    return null;
  }
  return null;
}

function normalizePathList(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  const seen = new Set<string>();
  const paths: string[] = [];
  for (const item of value) {
    if (typeof item !== "string") continue;
    const path = item.trim();
    if (!path || seen.has(path)) continue;
    seen.add(path);
    paths.push(path);
  }
  return paths;
}

export function parseSourceWorkspacePreference(
  value: string | null | undefined,
): SourceWorkspacePreference {
  const parsed = parseJsonRecord(value);
  const openPaths = normalizePathList(parsed?.openPaths);
  const rawActivePath = typeof parsed?.activePath === "string" ? parsed.activePath.trim() : "";
  const sideTab: SourceSideTabPreference = parsed?.sideTab === "search" ? "search" : "explorer";
  return {
    openPaths,
    expandedPaths: normalizePathList(parsed?.expandedPaths),
    activePath: rawActivePath || null,
    sideTab,
    preview: parsed?.preview === true,
    showHistory: parsed?.showHistory === true,
  };
}

export function serializeSourceWorkspacePreference(preference: SourceWorkspacePreference): string {
  return JSON.stringify({
    version: 1,
    openPaths: normalizePathList(preference.openPaths),
    expandedPaths: normalizePathList(preference.expandedPaths),
    activePath: preference.activePath || null,
    sideTab: preference.sideTab === "search" ? "search" : "explorer",
    preview: preference.preview,
    showHistory: preference.showHistory,
  });
}

export function parseGitWorkbenchPreference(
  value: string | null | undefined,
): GitWorkbenchPreference {
  const parsed = parseJsonRecord(value);
  const view = VALID_GIT_VIEWS.includes(parsed?.view as GitViewPreference)
    ? (parsed?.view as GitViewPreference)
    : "commits";
  const detailTab = VALID_GIT_DETAIL_TABS.includes(parsed?.detailTab as GitDetailTabPreference)
    ? (parsed?.detailTab as GitDetailTabPreference)
    : "commit";
  return {
    view,
    includeRemotes: parsed?.includeRemotes === true,
    includeTags: parsed?.includeTags === true,
    limit: clampNumberPreference(parsed?.limit, 200, { min: 50, max: 2000 }),
    detailTab,
  };
}

export function serializeGitWorkbenchPreference(preference: GitWorkbenchPreference): string {
  return JSON.stringify({
    version: 1,
    view: VALID_GIT_VIEWS.includes(preference.view) ? preference.view : "commits",
    includeRemotes: preference.includeRemotes,
    includeTags: preference.includeTags,
    limit: clampNumberPreference(preference.limit, 200, { min: 50, max: 2000 }),
    detailTab: VALID_GIT_DETAIL_TABS.includes(preference.detailTab)
      ? preference.detailTab
      : "commit",
  });
}
