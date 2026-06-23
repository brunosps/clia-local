import { describe, expect, it } from "vitest";
import { buildFileTree, fileTreeDirPaths } from "./patches";
import type { ChangedFile } from "./types";

function file(path: string): ChangedFile {
  return { path, status: "M", area: "unstaged", additions: 1, deletions: 0, can_stage_hunks: true };
}

describe("buildFileTree", () => {
  it("nests folders and keeps files as leaves", () => {
    const tree = buildFileTree([file("src/a.ts"), file("src/lib/b.ts"), file("README.md")]);
    // folders first (src), then files (README.md)
    expect(tree.map((n) => n.segment)).toEqual(["src", "README.md"]);
    const src = tree[0];
    expect(src.file).toBeUndefined();
    expect(src.path).toBe("src");
    // src children: folder lib first, then file a.ts
    expect(src.children.map((n) => n.segment)).toEqual(["lib", "a.ts"]);
    const leaf = src.children[1];
    expect(leaf.file?.path).toBe("src/a.ts");
  });

  it("collects all directory paths", () => {
    const tree = buildFileTree([file("a/b/c.ts"), file("a/d.ts")]);
    expect(fileTreeDirPaths(tree).sort()).toEqual(["a", "a/b"]);
  });
});
