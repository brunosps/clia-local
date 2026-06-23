import type { BlameLine } from "../types";

const UNCOMMITTED = "0000000000000000000000000000000000000000";

export function isUncommitted(line: BlameLine): boolean {
  return line.sha === UNCOMMITTED || /^0+$/.test(line.sha);
}

/** "agora", "3 dias atrás", "2 meses atrás" … from an ISO date. */
export function formatRelative(dateIso: string): string {
  const then = Date.parse(dateIso);
  if (Number.isNaN(then)) return "";
  const secs = Math.max(0, Math.floor((Date.now() - then) / 1000));
  const units: Array<[number, string, string]> = [
    [60, "segundo", "segundos"],
    [3600, "minuto", "minutos"],
    [86400, "hora", "horas"],
    [2592000, "dia", "dias"],
    [31536000, "mês", "meses"],
    [Infinity, "ano", "anos"],
  ];
  if (secs < 45) return "agora há pouco";
  let prev = 1;
  for (const [limit, sing, plur] of units) {
    if (secs < limit) {
      const value = Math.max(1, Math.floor(secs / prev));
      return `${value} ${value === 1 ? sing : plur} atrás`;
    }
    prev = limit;
  }
  return "";
}

/** Bucket suffix for gutter recency coloring (CSS class blame-age-<bucket>). */
export function recencyBucket(dateIso: string): string {
  const then = Date.parse(dateIso);
  if (Number.isNaN(then)) return "old";
  const days = (Date.now() - then) / 86400000;
  if (days < 7) return "0";
  if (days < 30) return "1";
  if (days < 180) return "2";
  if (days < 365) return "3";
  return "old";
}

/** End-of-line annotation text, GitLens-style. */
export function inlineBlameText(line: BlameLine): string {
  const rel = formatRelative(line.date);
  if (isUncommitted(line)) return `Não commitado ainda${rel ? `, ${rel}` : ""}`;
  return `${line.author}${rel ? `, ${rel}` : ""}${line.summary ? ` • ${line.summary}` : ""}`;
}

function uncommittedLabel(count: number): string {
  return count === 1 ? "1 linha não commitada" : `${count} linhas não commitadas`;
}

/** File-header CodeLens text: "N autores • última mudança por X, Y atrás". */
export function fileHeaderSummary(lines: BlameLine[]): string {
  const committed = lines.filter((line) => !isUncommitted(line));
  const uncommittedCount = lines.length - committed.length;
  if (committed.length === 0) {
    return uncommittedCount > 0 ? uncommittedLabel(uncommittedCount) : "Sem histórico de commits";
  }
  const authors = new Set(committed.map((line) => line.author)).size;
  // Most recent line by date.
  const latest = committed.reduce((best, line) =>
    Date.parse(line.date) > Date.parse(best.date) ? line : best,
  );
  const rel = formatRelative(latest.date);
  const authorLabel = authors === 1 ? "1 autor" : `${authors} autores`;
  const committedLabel = `${authorLabel} • última mudança por ${latest.author}${rel ? `, ${rel}` : ""}`;
  return uncommittedCount > 0
    ? `${uncommittedLabel(uncommittedCount)} • ${committedLabel}`
    : committedLabel;
}
