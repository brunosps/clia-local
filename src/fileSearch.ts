import type { SourceEntry } from "./types";

export interface FileHit {
  relative_path: string;
  name: string;
}

/** Flatten the source tree into a flat list of files (depth-first). */
export function flattenSourceFiles(entries: SourceEntry[]): FileHit[] {
  const out: FileHit[] = [];
  for (const entry of entries) {
    if (entry.kind === "file") out.push({ relative_path: entry.relative_path, name: entry.name });
    else out.push(...flattenSourceFiles(entry.children));
  }
  return out;
}

/**
 * Subsequence fuzzy score: returns null when `query` is not a subsequence of
 * `text`, otherwise a score where higher is better (rewards consecutive runs
 * and matches at word boundaries). Both args must be pre-lowercased.
 */
export function fuzzyScore(text: string, query: string): number | null {
  let ti = 0;
  let qi = 0;
  let score = 0;
  let streak = 0;
  while (ti < text.length && qi < query.length) {
    if (text[ti] === query[qi]) {
      qi += 1;
      streak += 1;
      score += streak;
      const prev = text[ti - 1];
      if (ti === 0 || prev === "/" || prev === "-" || prev === "_" || prev === ".") {
        score += 5;
      }
    } else {
      streak = 0;
    }
    ti += 1;
  }
  return qi === query.length ? score : null;
}

/** Rank flattened source files against a query (VSCode-style go-to-file). */
export function searchSourceFiles(
  entries: SourceEntry[],
  query: string,
  limit = 50,
): FileHit[] {
  const files = flattenSourceFiles(entries);
  const q = query.trim().toLowerCase();
  if (!q) {
    return [...files].sort((a, b) => a.relative_path.localeCompare(b.relative_path)).slice(0, limit);
  }
  const scored: { file: FileHit; score: number }[] = [];
  for (const file of files) {
    const nameScore = fuzzyScore(file.name.toLowerCase(), q);
    const pathScore = fuzzyScore(file.relative_path.toLowerCase(), q);
    let score: number | null = null;
    if (nameScore !== null) score = nameScore + 20; // prefer filename hits
    else if (pathScore !== null) score = pathScore;
    if (score !== null) scored.push({ file, score });
  }
  scored.sort(
    (a, b) =>
      b.score - a.score ||
      a.file.relative_path.length - b.file.relative_path.length ||
      a.file.name.localeCompare(b.file.name),
  );
  return scored.slice(0, limit).map((entry) => entry.file);
}
