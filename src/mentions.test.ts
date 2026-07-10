import { describe, expect, it } from "vitest";

import {
  basename,
  dedupeReferencedFiles,
  detectMention,
  insertMention,
  parseReferencedFiles,
  serializeReferencedFiles,
} from "./mentions";

describe("basename", () => {
  it("returns the last path segment for posix and windows separators", () => {
    expect(basename("WS/WSRETFPE.prw")).toBe("WSRETFPE.prw");
    expect(basename("a\\b\\C.tlpp")).toBe("C.tlpp");
    expect(basename("loose.prw")).toBe("loose.prw");
  });
});

describe("parseReferencedFiles / serializeReferencedFiles", () => {
  it("returns [] for empty, invalid, or non-array input", () => {
    expect(parseReferencedFiles(undefined)).toEqual([]);
    expect(parseReferencedFiles("")).toEqual([]);
    expect(parseReferencedFiles("not json")).toEqual([]);
    expect(parseReferencedFiles('{"path":"x"}')).toEqual([]);
  });

  it("normalizes entries, fills missing name from the path, and drops pathless ones", () => {
    const parsed = parseReferencedFiles(
      JSON.stringify([
        { name: "WSRETFPE.prw", path: "WS/WSRETFPE.prw" },
        { path: "SRC/A.tlpp" },
        { name: "ghost" },
      ]),
    );
    expect(parsed).toEqual([
      { name: "WSRETFPE.prw", path: "WS/WSRETFPE.prw" },
      { name: "A.tlpp", path: "SRC/A.tlpp" },
    ]);
  });

  it("de-duplicates by path keeping first occurrence", () => {
    const items = [
      { name: "A.prw", path: "x/A.prw" },
      { name: "A.prw", path: "x/A.prw" },
      { name: "B.prw", path: "y/B.prw" },
    ];
    expect(dedupeReferencedFiles(items)).toHaveLength(2);
    expect(parseReferencedFiles(serializeReferencedFiles(items))).toEqual([
      { name: "A.prw", path: "x/A.prw" },
      { name: "B.prw", path: "y/B.prw" },
    ]);
  });
});

describe("detectMention", () => {
  it("triggers on a bare @ at the start", () => {
    expect(detectMention("@", 1)).toEqual({ start: 0, query: "" });
  });

  it("captures the query after @ at the start of the text", () => {
    expect(detectMention("@WSR", 4)).toEqual({ start: 0, query: "WSR" });
  });

  it("triggers when the @ follows whitespace", () => {
    expect(detectMention("fix @WS", 7)).toEqual({ start: 4, query: "WS" });
    expect(detectMention("line1\n@A", 8)).toEqual({ start: 6, query: "A" });
  });

  it("does not trigger inside a word (e.g. an email)", () => {
    expect(detectMention("user@host", 9)).toBeNull();
  });

  it("closes the mention once a space is typed", () => {
    expect(detectMention("@foo ", 5)).toBeNull();
  });

  it("only considers the @ relative to the caret position", () => {
    // caret right after the @, query empty
    expect(detectMention("see @file.prw here", 5)).toEqual({ start: 4, query: "" });
    // caret in the middle of the query
    expect(detectMention("see @file.prw here", 9)).toEqual({ start: 4, query: "file" });
  });

  it("returns null when there is no @ before the caret", () => {
    expect(detectMention("plain text", 5)).toBeNull();
  });
});

describe("insertMention", () => {
  it("replaces the typed query with @<filename> and a trailing space", () => {
    const { text, caret } = insertMention("fix @WS", 4, 7, "WSRETFPE.prw");
    expect(text).toBe("fix @WSRETFPE.prw ");
    expect(caret).toBe(text.length);
  });

  it("keeps text after the caret intact", () => {
    const { text, caret } = insertMention("@A end", 0, 2, "ARQ.prw");
    expect(text).toBe("@ARQ.prw  end");
    expect(text.slice(caret)).toBe(" end");
  });
});
