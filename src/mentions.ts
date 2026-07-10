import type { ReferencedFile } from "./types";

/** The basename of a project-relative path (handles both `/` and `\`). */
export function basename(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || path;
}

/** De-duplicate referenced files by `path`, preserving first-seen order. */
export function dedupeReferencedFiles(items: ReferencedFile[]): ReferencedFile[] {
  const seen = new Set<string>();
  const out: ReferencedFile[] = [];
  for (const item of items) {
    if (!item?.path || seen.has(item.path)) continue;
    seen.add(item.path);
    out.push({ name: item.name || basename(item.path), path: item.path });
  }
  return out;
}

/** Parse a card's `referenced_files_json` into a normalized, de-duplicated list. */
export function parseReferencedFiles(json: string | null | undefined): ReferencedFile[] {
  if (!json) return [];
  try {
    const parsed: unknown = JSON.parse(json);
    if (!Array.isArray(parsed)) return [];
    const items: ReferencedFile[] = [];
    for (const entry of parsed) {
      if (!entry || typeof entry !== "object") continue;
      const record = entry as Record<string, unknown>;
      const path = typeof record.path === "string" ? record.path : "";
      if (!path) continue;
      const name = typeof record.name === "string" && record.name ? record.name : basename(path);
      items.push({ name, path });
    }
    return dedupeReferencedFiles(items);
  } catch {
    return [];
  }
}

export function serializeReferencedFiles(items: ReferencedFile[]): string {
  return JSON.stringify(dedupeReferencedFiles(items));
}

/**
 * Detect an in-progress `@` mention ending at the caret. Returns the index of the
 * `@` and the query typed after it (without the `@`), or `null` when the caret is
 * not inside a fresh mention.
 *
 * A mention only triggers when the `@` sits at the start of the text or right after
 * whitespace (so `user@host` does not trigger), and the run between `@` and the
 * caret must not contain whitespace (a space closes the mention).
 */
export function detectMention(
  text: string,
  caret: number,
): { start: number; query: string } | null {
  if (caret < 0 || caret > text.length) return null;
  let i = caret - 1;
  while (i >= 0) {
    const ch = text[i];
    if (ch === "@") {
      const before = i === 0 ? "" : text[i - 1];
      if (before === "" || /\s/.test(before)) {
        return { start: i, query: text.slice(i + 1, caret) };
      }
      return null;
    }
    if (/\s/.test(ch)) return null;
    i -= 1;
  }
  return null;
}

/**
 * Replace the mention being typed (from `start`, the `@`, up to `caret`) with
 * `@<filename> ` and return the new text plus the caret position after the inserted
 * token (including the trailing space).
 */
export function insertMention(
  text: string,
  start: number,
  caret: number,
  filename: string,
): { text: string; caret: number } {
  const token = `@${filename} `;
  const next = text.slice(0, start) + token + text.slice(caret);
  return { text: next, caret: start + token.length };
}
