import { describe, expect, it } from "vitest";
import {
  fileHeaderSummary,
  formatRelative,
  inlineBlameText,
  isUncommitted,
  recencyBucket,
} from "./gitlens";
import type { BlameLine } from "../types";

function line(over: Partial<BlameLine>): BlameLine {
  return {
    line: 1,
    sha: "a".repeat(40),
    short_sha: "aaaaaaaa",
    author: "Bruno",
    author_email: "b@x.com",
    date: new Date().toISOString(),
    summary: "feat: do it",
    ...over,
  };
}

describe("formatRelative", () => {
  it("returns recent + day buckets", () => {
    expect(formatRelative(new Date().toISOString())).toContain("agora");
    const threeDays = new Date(Date.now() - 3 * 86400000).toISOString();
    expect(formatRelative(threeDays)).toBe("3 dias atrás");
  });
  it("returns empty on bad input", () => {
    expect(formatRelative("not-a-date")).toBe("");
  });
});

describe("recencyBucket", () => {
  it("buckets by age", () => {
    expect(recencyBucket(new Date().toISOString())).toBe("0");
    expect(recencyBucket(new Date(Date.now() - 60 * 86400000).toISOString())).toBe("2");
    expect(recencyBucket(new Date(Date.now() - 800 * 86400000).toISOString())).toBe("old");
  });
});

describe("inlineBlameText / isUncommitted", () => {
  it("formats author + relative + summary", () => {
    const text = inlineBlameText(line({ date: new Date(Date.now() - 86400000).toISOString() }));
    expect(text).toContain("Bruno");
    expect(text).toContain("feat: do it");
  });
  it("labels uncommitted lines", () => {
    const u = line({ sha: "0".repeat(40), short_sha: "00000000" });
    expect(isUncommitted(u)).toBe(true);
    expect(inlineBlameText(u)).toBe("Não commitado ainda, agora há pouco");
  });

  it("keeps the uncommitted label when timestamp is unavailable", () => {
    const u = line({ sha: "0".repeat(40), short_sha: "00000000", date: "" });
    expect(inlineBlameText(u)).toBe("Não commitado ainda");
  });
});

describe("fileHeaderSummary", () => {
  it("counts authors and reports the latest change", () => {
    const lines = [
      line({ author: "Bruno", date: new Date(Date.now() - 5 * 86400000).toISOString() }),
      line({
        author: "Ana",
        sha: "b".repeat(40),
        date: new Date(Date.now() - 86400000).toISOString(),
      }),
    ];
    const summary = fileHeaderSummary(lines);
    expect(summary).toContain("2 autores");
    expect(summary).toContain("Ana");
  });

  it("mentions uncommitted lines without dropping committed history", () => {
    const summary = fileHeaderSummary([
      line({ author: "Bruno", date: new Date(Date.now() - 86400000).toISOString() }),
      line({ sha: "0".repeat(40), short_sha: "00000000" }),
    ]);

    expect(summary).toContain("1 linha não commitada");
    expect(summary).toContain("1 autor");
  });

  it("summarizes files with only uncommitted lines", () => {
    const summary = fileHeaderSummary([
      line({ sha: "0".repeat(40), short_sha: "00000000" }),
      line({ sha: "0".repeat(40), short_sha: "00000000" }),
    ]);

    expect(summary).toBe("2 linhas não commitadas");
  });
});
