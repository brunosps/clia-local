import { describe, expect, it } from "vitest";
import {
  GIT_SNAPSHOT_AUTO_FETCH_COOLDOWN_MS,
  gitAutoFetchKey,
  gitSnapshotCacheKey,
  reconcileChangedFileSelection,
  parseGitSnapshotCache,
  resolveAutoFetchRemote,
  shouldLoadWorktreeSnapshot,
  shouldAutoFetch,
} from "./gitSnapshot";
import type { ChangedFile, GitRepoSnapshot, RemoteBranch, RepoState } from "./types";

const repoState: RepoState = {
  branch: "main",
  detached: false,
  upstream: "origin/main",
  ahead: 0,
  behind: 0,
  operation: null,
  conflicts: [],
  dirty: false,
};

const snapshot: GitRepoSnapshot = {
  commits: [],
  branches: [],
  remote_branches: [],
  tags: [],
  stashes: [],
  submodules: [],
  repo_state: repoState,
  generated_at: "2026-05-28T10:00:00Z",
  options: { include_remotes: false, include_tags: false, limit: 200 },
  warnings: [],
};

describe("git snapshot cache keys", () => {
  it("includes project and graph options", () => {
    expect(gitSnapshotCacheKey(7, { includeRemotes: true, includeTags: false, limit: 400 })).toBe(
      "git_snapshot:7:true:false:400",
    );
    expect(gitAutoFetchKey(7)).toBe("git_auto_fetch_at:7");
  });
});

describe("parseGitSnapshotCache", () => {
  it("returns a snapshot for valid serialized cache", () => {
    expect(parseGitSnapshotCache(JSON.stringify(snapshot))?.repo_state.branch).toBe("main");
  });

  it("rejects invalid or incomplete cache entries", () => {
    expect(parseGitSnapshotCache("not json")).toBeNull();
    expect(parseGitSnapshotCache(JSON.stringify({ ...snapshot, commits: undefined }))).toBeNull();
  });
});

describe("resolveAutoFetchRemote", () => {
  it("prefers the current upstream remote", () => {
    const branches: RemoteBranch[] = [{ remote: "fork", name: "main", full: "fork/main" }];
    expect(resolveAutoFetchRemote(repoState, branches)).toBe("origin");
  });

  it("falls back to origin or a single clear remote", () => {
    expect(
      resolveAutoFetchRemote(null, [{ remote: "origin", name: "main", full: "origin/main" }]),
    ).toBe("origin");
    expect(resolveAutoFetchRemote(null, [{ remote: "up", name: "main", full: "up/main" }])).toBe(
      "up",
    );
    expect(
      resolveAutoFetchRemote(null, [
        { remote: "up", name: "main", full: "up/main" },
        { remote: "fork", name: "main", full: "fork/main" },
      ]),
    ).toBeNull();
  });
});

describe("shouldAutoFetch", () => {
  it("fetches when there is a remote and no previous fetch", () => {
    expect(shouldAutoFetch({ now: 1000, lastFetchAt: null, busy: false, remote: "origin" })).toBe(
      true,
    );
  });

  it("respects busy state, operations, missing remotes and cooldown", () => {
    const now = GIT_SNAPSHOT_AUTO_FETCH_COOLDOWN_MS * 2;
    expect(shouldAutoFetch({ now, lastFetchAt: null, busy: true, remote: "origin" })).toBe(false);
    expect(
      shouldAutoFetch({
        now,
        lastFetchAt: null,
        busy: false,
        operation: "rebase",
        remote: "origin",
      }),
    ).toBe(false);
    expect(shouldAutoFetch({ now, lastFetchAt: null, busy: false, remote: null })).toBe(false);
    expect(
      shouldAutoFetch({
        now,
        lastFetchAt: now - GIT_SNAPSHOT_AUTO_FETCH_COOLDOWN_MS + 1,
        busy: false,
        remote: "origin",
      }),
    ).toBe(false);
    expect(
      shouldAutoFetch({
        now,
        lastFetchAt: now - GIT_SNAPSHOT_AUTO_FETCH_COOLDOWN_MS,
        busy: false,
        remote: "origin",
      }),
    ).toBe(true);
  });
});

describe("worktree refresh helpers", () => {
  const files: ChangedFile[] = [
    {
      path: "src/a.ts",
      old_path: null,
      status: "M",
      area: "unstaged",
      additions: 1,
      deletions: 0,
      can_stage_hunks: true,
    },
    {
      path: "src/b.ts",
      old_path: null,
      status: "M",
      area: "staged",
      additions: 2,
      deletions: 1,
      can_stage_hunks: true,
    },
  ];

  it("preserves the selected changed file when it survives refresh", () => {
    expect(
      reconcileChangedFileSelection({
        files,
        selected: { ...files[1] },
        autoSelect: false,
      }),
    ).toEqual(files[1]);
  });

  it("does not auto-select another file during background refresh", () => {
    expect(
      reconcileChangedFileSelection({
        files,
        selected: {
          ...files[0],
          path: "deleted.ts",
        },
        autoSelect: false,
      }),
    ).toBeNull();
  });

  it("auto-selects the first file during explicit refresh", () => {
    expect(reconcileChangedFileSelection({ files, selected: null, autoSelect: true })).toEqual(
      files[0],
    );
  });

  it("loads a snapshot only when there is no fingerprint or it changed", () => {
    expect(shouldLoadWorktreeSnapshot(null, "next")).toBe(true);
    expect(shouldLoadWorktreeSnapshot("current", null)).toBe(true);
    expect(shouldLoadWorktreeSnapshot("same", "same")).toBe(false);
    expect(shouldLoadWorktreeSnapshot("old", "new")).toBe(true);
  });
});
