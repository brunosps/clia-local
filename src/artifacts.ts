import type { DwArtifact } from "./types";

export function artifactCounts(artifacts: DwArtifact[]) {
  return artifacts.reduce<Record<DwArtifact["category"], number>>(
    (counts, artifact) => {
      counts[artifact.category] += 1;
      return counts;
    },
    { state: 0, spec: 0, bugfix: 0, command: 0, rule: 0, support: 0 },
  );
}

export function artifactLanguage(relativePath: string) {
  return relativePath.endsWith(".json") ? "json" : "markdown";
}

export function formatArtifactSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  return `${(bytes / 1024).toFixed(1)} KB`;
}
