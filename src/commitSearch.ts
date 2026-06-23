import type { Commit } from "./types";

/** Case-insensitive match of a commit against a query (subject/author/email/sha). */
export function matchCommit(commit: Commit, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  return (
    commit.subject.toLowerCase().includes(q) ||
    commit.author_name.toLowerCase().includes(q) ||
    commit.author_email.toLowerCase().includes(q) ||
    commit.sha.toLowerCase().includes(q) ||
    commit.short_sha.toLowerCase().includes(q)
  );
}

export function filterCommits(commits: Commit[], query: string): Commit[] {
  if (!query.trim()) return commits;
  return commits.filter((commit) => matchCommit(commit, query));
}
