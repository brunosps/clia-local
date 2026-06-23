import type { Project, Workspace } from "./types";

export const defaultWorkspaceName = "Local";

export function parseStateId(value: string | null): number | null {
  if (value === null) return null;
  const parsed = Number(value.trim());
  return Number.isInteger(parsed) && parsed > 0 ? parsed : null;
}

export function pickActiveWorkspace(
  workspaces: Workspace[],
  lastId: number | null,
): Workspace | null {
  if (lastId !== null) {
    const remembered = workspaces.find((workspace) => workspace.id === lastId);
    if (remembered) return remembered;
  }
  return workspaces[0] ?? null;
}

export function pickActiveProject(projects: Project[], lastId: number | null): Project | null {
  if (lastId !== null) {
    const remembered = projects.find((project) => project.id === lastId);
    if (remembered) return remembered;
  }
  return projects[0] ?? null;
}

export function createDefaultWorkspaceRoot(projectPath: string) {
  const trimmed = projectPath.trim();
  return trimmed || ".";
}

export function projectDisplayName(project: Project) {
  const trimmedName = project.name.trim();
  return trimmedName || project.path;
}
