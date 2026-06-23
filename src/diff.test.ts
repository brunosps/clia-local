import { describe, expect, it } from "vitest";
import { parsePatchToBlocks, parseUnifiedRows, tokenize } from "./diff";
import type { FilePatch } from "./types";

describe("parseUnifiedRows", () => {
  it("assigns old/new line numbers and row types", () => {
    const patch = ["@@ -1,3 +1,4 @@", " context", "-removed", "+added one", "+added two", " tail"].join(
      "\n",
    );
    const rows = parseUnifiedRows(patch);
    expect(rows[0]).toMatchObject({ type: "hunk" });
    expect(rows[1]).toMatchObject({ type: "context", oldNo: 1, newNo: 1, content: "context" });
    expect(rows[2]).toMatchObject({ type: "del", oldNo: 2, newNo: null, content: "removed" });
    expect(rows[3]).toMatchObject({ type: "add", oldNo: null, newNo: 2, content: "added one" });
    expect(rows[4]).toMatchObject({ type: "add", oldNo: null, newNo: 3, content: "added two" });
    expect(rows[5]).toMatchObject({ type: "context", oldNo: 3, newNo: 4, content: "tail" });
  });

  it("treats file headers and no-newline markers as meta", () => {
    const rows = parseUnifiedRows(
      ["diff --git a/x b/x", "index 111..222 100644", "--- a/x", "+++ b/x", "@@ -1 +1 @@", "+y", "\\ No newline at end of file"].join("\n"),
    );
    expect(rows.filter((r) => r.type === "meta")).toHaveLength(5);
    expect(rows.find((r) => r.type === "add")?.content).toBe("y");
  });
});

describe("parsePatchToBlocks", () => {
  it("makes one block per hunk when structured hunks exist", () => {
    const patch: FilePatch = {
      path: "a.ts",
      area: "unstaged",
      patch: "",
      hunks: [
        { id: "h1", header: "@@ -1 +1 @@", old_start: 1, old_lines: 1, new_start: 1, new_lines: 1, patch: "@@ -1 +1 @@\n-a\n+b" },
        { id: "h2", header: "@@ -5 +5 @@", old_start: 5, old_lines: 1, new_start: 5, new_lines: 1, patch: "@@ -5 +5 @@\n-c\n+d" },
      ],
    };
    const blocks = parsePatchToBlocks(patch);
    expect(blocks).toHaveLength(2);
    expect(blocks[0].hunk?.id).toBe("h1");
    expect(blocks[1].rows.some((r) => r.type === "add" && r.content === "d")).toBe(true);
  });

  it("falls back to a single block over the whole patch", () => {
    const patch: FilePatch = { path: "a.ts", area: "unstaged", patch: "@@ -1 +1 @@\n-a\n+b", hunks: [] };
    const blocks = parsePatchToBlocks(patch);
    expect(blocks).toHaveLength(1);
    expect(blocks[0].hunk).toBeNull();
  });
});

describe("tokenize", () => {
  it("classifies keywords, strings and comments for js", () => {
    const tokens = tokenize('const x = "hi"; // note', "javascript");
    expect(tokens.find((t) => t.text === "const")?.cls).toBe("kw");
    expect(tokens.find((t) => t.text === '"hi"')?.cls).toBe("str");
    expect(tokens.find((t) => t.cls === "com")?.text).toContain("// note");
  });

  it("returns plain text for non-tokenized languages", () => {
    const tokens = tokenize("# heading", "markdown");
    expect(tokens).toEqual([{ text: "# heading", cls: "" }]);
  });
});
