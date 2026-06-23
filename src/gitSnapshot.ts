import type { ChangedFile, GitRepoSnapshot, RemoteBranch, RepoState } from "./types";

export const GIT_SNAPSHOT_AUTO_FETCH_COOLDOWN_MS = 5 * 60 * 1000;

export type GitSnapshotOptions = {
  includeRemotes: boolean;
  includeTags: boolean;
  limit: number;
};

export function gitSnapshotCacheKey(projectId: number, options: GitSnapshotOptions) {
  return `git_snapshot:${projectId}:${options.includeRemotes}:${options.includeTags}:${options.limit}`;
}

export function gitAutoFetchKey(projectId: number) {
  return `git_auto_fetch_at:${projectId}`;
}

export function parseGitSnapshotCache(value: string | null): GitRepoSnapshot | null {
  if (!value) return null;
  try {
    const parsed = JSON.parse(value) as Partial<GitRepoSnapshot>;
    if (
      !parsed ||
      typeof parsed !== "object" ||
      !Array.isArray(parsed.commits) ||
      !Array.isArray(parsed.branches) ||
      !Array.isArray(parsed.remote_branches) ||
      !Array.isArray(parsed.tags) ||
      !Array.isArray(parsed.stashes) ||
      !Array.isArray(parsed.submodules) ||
      !parsed.repo_state ||
      typeof parsed.generated_at !== "string"
    ) {
      return null;
    }
    return parsed as GitRepoSnapshot;
  } catch {
    return null;
  }
}

export function serializeGitSnapshotCache(snapshot: GitRepoSnapshot) {
  return JSON.stringify(snapshot);
}

export function resolveAutoFetchRemote(
  repoState: RepoState | null,
  remoteBranches: RemoteBranch[],
) {
  const upstreamRemote = repoState?.upstream?.split("/")[0]?.trim();
  if (upstreamRemote) return upstreamRemote;

  const remotes = Array.from(
    new Set(remoteBranches.map((branch) => branch.remote).filter(Boolean)),
  );
  if (remotes.includes("origin")) return "origin";
  return remotes.length === 1 ? remotes[0] : null;
}

export function shouldAutoFetch({
  now,
  lastFetchAt,
  busy,
  operation,
  remote,
  cooldownMs = GIT_SNAPSHOT_AUTO_FETCH_COOLDOWN_MS,
}: {
  now: number;
  lastFetchAt: number | null;
  busy: boolean;
  operation?: string | null;
  remote?: string | null;
  cooldownMs?: number;
}) {
  if (busy || operation || !remote) return false;
  if (lastFetchAt == null) return true;
  return now - lastFetchAt >= cooldownMs;
}

export function sameChangedFile(left: ChangedFile | null, right: ChangedFile | null) {
  return Boolean(left && right && left.path === right.path && left.area === right.area);
}

export function findChangedFile(files: ChangedFile[], selected: ChangedFile | null) {
  if (!selected) return null;
  return files.find((file) => file.path === selected.path && file.area === selected.area) ?? null;
}

export function reconcileChangedFileSelection({
  files,
  selected,
  autoSelect,
}: {
  files: ChangedFile[];
  selected: ChangedFile | null;
  autoSelect: boolean;
}) {
  const stillSelected = findChangedFile(files, selected);
  return stillSelected ?? (autoSelect ? (files[0] ?? null) : null);
}

export function shouldLoadWorktreeSnapshot(
  currentFingerprint: string | null,
  nextFingerprint: string | null,
) {
  return !currentFingerprint || !nextFingerprint || currentFingerprint !== nextFingerprint;
}
