import { describe, expect, it } from "vitest";
import { flattenSourceFiles, fuzzyScore, searchSourceFiles } from "./fileSearch";
import type { SourceEntry } from "./types";

function file(relative_path: string, name: string): SourceEntry {
  return { relative_path, name, kind: "file", children: [] };
}
function dir(relative_path: string, name: string, children: SourceEntry[]): SourceEntry {
  return { relative_path, name, kind: "directory", children };
}

const tree: SourceEntry[] = [
  dir("src", "src", [
    file("src/diff.ts", "diff.ts"),
    file("src/diff.test.ts", "diff.test.ts"),
    dir("src/source", "source", [file("src/source/DiffCompare.tsx", "DiffCompare.tsx")]),
  ]),
  file("README.md", "README.md"),
];

describe("flattenSourceFiles", () => {
  it("collects every file, skipping directories", () => {
    expect(flattenSourceFiles(tree).map((f) => f.relative_path)).toEqual([
      "src/diff.ts",
      "src/diff.test.ts",
      "src/source/DiffCompare.tsx",
      "README.md",
    ]);
  });
});

describe("fuzzyScore", () => {
  it("returns null when the query is not a subsequence", () => {
    expect(fuzzyScore("diff.ts", "xyz")).toBeNull();
  });

  it("scores consecutive matches higher than scattered ones", () => {
    const consecutive = fuzzyScore("diff.ts", "diff");
    const scattered = fuzzyScore("dxixfxf", "diff");
    expect(consecutive).not.toBeNull();
    expect(scattered).not.toBeNull();
    expect(consecutive!).toBeGreaterThan(scattered!);
  });
});

describe("searchSourceFiles", () => {
  it("ranks filename matches above path-only matches", () => {
    const hits = searchSourceFiles(tree, "diff");
    expect(hits[0].relative_path).toBe("src/diff.ts");
    expect(hits.map((h) => h.relative_path)).toContain("src/source/DiffCompare.tsx");
  });

  it("returns all files sorted by path when the query is empty", () => {
    expect(searchSourceFiles(tree, "  ").map((h) => h.name)).toEqual([
      "README.md",
      "diff.test.ts",
      "diff.ts",
      "DiffCompare.tsx",
    ]);
  });

  it("respects the limit", () => {
    expect(searchSourceFiles(tree, "", 2)).toHaveLength(2);
  });
});
