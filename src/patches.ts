import type { ChangedFile, PatchArea } from "./types";

export type PatchGroups = {
  staged: ChangedFile[];
  unstaged: ChangedFile[];
};

export function groupChangedFiles(files: ChangedFile[]): PatchGroups {
  return {
    staged: files.filter((file) => file.area === "staged"),
    unstaged: files.filter((file) => file.area === "unstaged"),
  };
}

export function countPatchAreas(files: ChangedFile[]) {
  const groups = groupChangedFiles(files);
  return { staged: groups.staged.length, unstaged: groups.unstaged.length };
}

export function formatPatchStats(file: ChangedFile) {
  return `+${file.additions} -${file.deletions}`;
}

export function patchAreaLabel(area: PatchArea) {
  return area === "staged" ? "Staged" : "Unstaged";
}

export function fileActionLabel(file: ChangedFile) {
  return file.area === "staged" ? "Unstage file" : "Stage file";
}

export function hunkActionLabel(file: ChangedFile) {
  return file.area === "staged" ? "Unstage hunk" : "Stage hunk";
}

export function canStageHunks(file: ChangedFile | null) {
  return Boolean(file?.can_stage_hunks);
}

// ---------------------------------------------------------------------------
// File tree for the Local Changes panes (Fork-style nested folders).
// ---------------------------------------------------------------------------

export type FileTreeNode = {
  segment: string;
  path: string; // folder path or full file path
  file?: ChangedFile; // set on leaves
  children: FileTreeNode[];
};

/** Build a nested folder tree from changed-file paths (folders before files). */
export function buildFileTree(files: ChangedFile[]): FileTreeNode[] {
  const roots: FileTreeNode[] = [];
  for (const file of files) {
    const segments = file.path.split("/").filter(Boolean);
    let level = roots;
    let prefix = "";
    segments.forEach((segment, index) => {
      prefix = prefix ? `${prefix}/${segment}` : segment;
      const isLeaf = index === segments.length - 1;
      let node = level.find((candidate) => candidate.segment === segment && !candidate.file);
      if (isLeaf) {
        level.push({ segment, path: file.path, file, children: [] });
        return;
      }
      if (!node) {
        node = { segment, path: prefix, children: [] };
        level.push(node);
      }
      level = node.children;
    });
  }
  const sort = (nodes: FileTreeNode[]) => {
    nodes.sort((a, b) => {
      const aDir = a.file ? 1 : 0;
      const bDir = b.file ? 1 : 0;
      if (aDir !== bDir) return aDir - bDir; // folders first
      return a.segment.localeCompare(b.segment);
    });
    for (const node of nodes) if (!node.file) sort(node.children);
  };
  sort(roots);
  return roots;
}

/** Every folder path in a file tree (for "expand all" defaults). */
export function fileTreeDirPaths(nodes: FileTreeNode[]): string[] {
  const paths: string[] = [];
  for (const node of nodes) {
    if (!node.file) {
      paths.push(node.path);
      paths.push(...fileTreeDirPaths(node.children));
    }
  }
  return paths;
}
