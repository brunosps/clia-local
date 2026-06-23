import type { FilePatch, PatchHunk } from "./types";
import type { SourceLanguage } from "./source";

// TEMP: edição de demonstração para visualizar o diff colorido no Local Changes.
// Pode descartar este comentário pelo botão "Descartar".

// ---------------------------------------------------------------------------
// Unified-diff parsing for the colored diff renderer.
// ---------------------------------------------------------------------------

export type DiffRowType = "add" | "del" | "context" | "hunk" | "meta";

export interface DiffRow {
  type: DiffRowType;
  /** 1-based old-file line number, or null for adds/hunk/meta. */
  oldNo: number | null;
  /** 1-based new-file line number, or null for dels/hunk/meta. */
  newNo: number | null;
  /** Line content WITHOUT the leading +/-/space marker (full text for hunk/meta). */
  content: string;
}

export interface DiffBlock {
  /** The hunk this block maps to (for stage/unstage/discard), or null for a whole-file render. */
  hunk: PatchHunk | null;
  rows: DiffRow[];
}

const HUNK_HEADER = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/;

/** Parse a unified patch (one or many hunks, with optional file headers) into rows. */
export function parseUnifiedRows(patch: string): DiffRow[] {
  const rows: DiffRow[] = [];
  let oldNo = 0;
  let newNo = 0;
  // Split keeping no trailing empty line noise; a patch line keeps its marker.
  const lines = patch.replace(/\n$/, "").split("\n");
  for (const line of lines) {
    const header = HUNK_HEADER.exec(line);
    if (header) {
      oldNo = Number(header[1]);
      newNo = Number(header[2]);
      rows.push({ type: "hunk", oldNo: null, newNo: null, content: line });
      continue;
    }
    if (
      line.startsWith("diff ") ||
      line.startsWith("index ") ||
      line.startsWith("--- ") ||
      line.startsWith("+++ ") ||
      line.startsWith("old mode") ||
      line.startsWith("new mode") ||
      line.startsWith("similarity ") ||
      line.startsWith("rename ") ||
      line.startsWith("\\")
    ) {
      rows.push({ type: "meta", oldNo: null, newNo: null, content: line });
      continue;
    }
    const marker = line[0] ?? " ";
    const body = line.slice(1);
    if (marker === "+") {
      rows.push({ type: "add", oldNo: null, newNo, content: body });
      newNo += 1;
    } else if (marker === "-") {
      rows.push({ type: "del", oldNo, newNo: null, content: body });
      oldNo += 1;
    } else {
      rows.push({ type: "context", oldNo, newNo, content: body });
      oldNo += 1;
      newNo += 1;
    }
  }
  return rows;
}

/**
 * Split a file patch into blocks. When the backend gave structured hunks, one
 * block per hunk (so stage/unstage/discard buttons map 1:1 to `hunk.patch`);
 * otherwise a single block over the whole patch.
 */
export function parsePatchToBlocks(patch: FilePatch): DiffBlock[] {
  if (patch.hunks.length > 0) {
    return patch.hunks.map((hunk) => ({ hunk, rows: parseUnifiedRows(hunk.patch) }));
  }
  return [{ hunk: null, rows: parseUnifiedRows(patch.patch) }];
}

// ---------------------------------------------------------------------------
// Best-effort per-line syntax tokenizer (correctness load is on +/-/context
// coloring; tokens are decorative and must never throw).
// ---------------------------------------------------------------------------

export type TokenClass = "kw" | "str" | "com" | "num" | "";

export interface Token {
  text: string;
  cls: TokenClass;
}

const KEYWORDS: Partial<Record<SourceLanguage, Set<string>>> = {
  javascript: new Set([
    "const", "let", "var", "function", "return", "if", "else", "for", "while", "switch",
    "case", "break", "continue", "new", "class", "extends", "import", "export", "from",
    "default", "async", "await", "try", "catch", "finally", "throw", "typeof", "instanceof",
    "this", "super", "null", "undefined", "true", "false", "void", "yield", "interface",
    "type", "enum", "as", "in", "of",
  ]),
  rust: new Set([
    "fn", "let", "mut", "const", "static", "struct", "enum", "impl", "trait", "pub", "use",
    "mod", "match", "if", "else", "for", "while", "loop", "return", "self", "Self", "super",
    "crate", "where", "async", "await", "move", "ref", "dyn", "as", "in", "true", "false",
    "Some", "None", "Ok", "Err", "unsafe", "type",
  ]),
  json: new Set(["true", "false", "null"]),
};

// strings | line comments | numbers | identifiers
const TOKEN_RE =
  /("(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|`(?:\\.|[^`\\])*`)|(\/\/.*|#.*)|(\b\d[\d_.eExXa-fA-F+-]*\b)|([A-Za-z_$][\w$]*)/g;

export function tokenize(content: string, lang: SourceLanguage): Token[] {
  const keywords = KEYWORDS[lang];
  if (!content) return [{ text: "", cls: "" }];
  // Languages we don't tokenize: render plain (markdown/css/html/plain).
  if (!keywords && lang !== "json") return [{ text: content, cls: "" }];

  const tokens: Token[] = [];
  let last = 0;
  TOKEN_RE.lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = TOKEN_RE.exec(content)) !== null) {
    if (match.index > last) tokens.push({ text: content.slice(last, match.index), cls: "" });
    if (match[1]) tokens.push({ text: match[1], cls: "str" });
    else if (match[2]) tokens.push({ text: match[2], cls: "com" });
    else if (match[3]) tokens.push({ text: match[3], cls: "num" });
    else if (match[4]) {
      tokens.push({ text: match[4], cls: keywords?.has(match[4]) ? "kw" : "" });
    }
    last = TOKEN_RE.lastIndex;
  }
  if (last < content.length) tokens.push({ text: content.slice(last), cls: "" });
  return tokens.length ? tokens : [{ text: content, cls: "" }];
}
