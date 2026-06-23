import { describe, expect, it } from "vitest";
import { filterCommits, matchCommit } from "./commitSearch";
import type { Commit } from "./types";

const base: Commit = {
  sha: "abcdef1234567890",
  short_sha: "abcdef1",
  parents: [],
  refs: [],
  author_name: "Bruno Santos",
  author_email: "bruno@example.com",
  date: "2026-05-26",
  subject: "feat(git): add colored diff",
};

describe("matchCommit", () => {
  it("matches blank query", () => {
    expect(matchCommit(base, "")).toBe(true);
  });
  it("matches subject / author / email / sha (case-insensitive)", () => {
    expect(matchCommit(base, "COLORED")).toBe(true);
    expect(matchCommit(base, "bruno")).toBe(true);
    expect(matchCommit(base, "example.com")).toBe(true);
    expect(matchCommit(base, "abcdef1")).toBe(true);
  });
  it("does not match unrelated text", () => {
    expect(matchCommit(base, "submodule")).toBe(false);
  });
});

describe("filterCommits", () => {
  it("returns all on blank query and filters otherwise", () => {
    const list = [base, { ...base, subject: "fix: bug", sha: "z", short_sha: "z" }];
    expect(filterCommits(list, "")).toHaveLength(2);
    expect(filterCommits(list, "bug")).toHaveLength(1);
  });
});
