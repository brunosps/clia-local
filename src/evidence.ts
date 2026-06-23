import type { EvidenceEntry } from "./types";

export function evidenceCounts(entries: EvidenceEntry[]) {
  return entries.reduce(
    (counts, entry) => {
      if (entry.record_type === "run") counts.runs += 1;
      else counts.items += 1;
      if (entry.status === "submitted") counts.submitted += 1;
      if (entry.stale || entry.status === "stale") counts.stale += 1;
      return counts;
    },
    { runs: 0, items: 0, submitted: 0, stale: 0 },
  );
}

export function evidenceStatusLabel(status: string) {
  switch (status) {
    case "submitted":
      return "Submitted";
    case "passed":
      return "Passed";
    case "failed":
      return "Failed";
    case "unknown":
      return "Unknown";
    case "indexed":
      return "Indexed";
    case "stale":
      return "Stale";
    default:
      return status;
  }
}

export function evidenceKindLabel(kind: string) {
  switch (kind) {
    case "qa-report":
      return "QA report";
    case "bug-report":
      return "Bug report";
    case "run-log":
      return "Run log";
    default:
      return kind.replace(/-/g, " ");
  }
}

export function parseEvidenceLinks(value: string) {
  return value
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
}

export function evidenceSummary(entry: EvidenceEntry) {
  if (entry.summary.trim()) return entry.summary.trim();
  if (entry.relative_path) return entry.relative_path;
  return entry.command ?? entry.title;
}
