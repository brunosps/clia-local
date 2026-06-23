import type { SourceEntry } from "./types";

export type SourceLanguage = "javascript" | "json" | "markdown" | "rust" | "css" | "html" | "plain";

export function sourceLanguage(relativePath: string): SourceLanguage {
  const extension = relativePath.split(".").pop()?.toLowerCase();

  if (["ts", "tsx", "js", "jsx", "mjs", "cjs"].includes(extension ?? "")) {
    return "javascript";
  }

  if (extension === "json") return "json";
  if (extension === "md" || extension === "mdx") return "markdown";
  if (extension === "rs") return "rust";
  if (extension === "css") return "css";
  if (extension === "html" || extension === "htm") return "html";
  return "plain";
}

/** Monaco language id for a path (richer than the diff tokenizer's SourceLanguage). */
export function monacoLanguage(relativePath: string): string {
  const name = relativePath.split("/").pop()?.toLowerCase() ?? "";
  const ext = name.split(".").pop() ?? "";
  if (["ts", "tsx", "mts", "cts"].includes(ext)) return "typescript";
  if (["js", "jsx", "mjs", "cjs"].includes(ext)) return "javascript";
  if (ext === "json" || name === ".prettierrc" || name === "tsconfig.json") return "json";
  if (ext === "md" || ext === "mdx") return "markdown";
  if (ext === "rs") return "rust";
  if (ext === "cs") return "csharp";
  if (ext === "css") return "css";
  if (ext === "scss") return "scss";
  if (ext === "less") return "less";
  if (ext === "html" || ext === "htm") return "html";
  if (ext === "py" || ext === "pyi") return "python";
  if (["yml", "yaml"].includes(ext)) return "yaml";
  if (ext === "toml" || name === "cargo.lock") return "ini";
  if (["sh", "bash", "zsh"].includes(ext) || name === ".bashrc") return "shell";
  if (ext === "sql") return "sql";
  if (ext === "xml" || ext === "svg") return "xml";
  if (ext === "go") return "go";
  if (ext === "java") return "java";
  if (ext === "c" || ext === "h") return "c";
  if (["cpp", "cc", "hpp"].includes(ext)) return "cpp";
  if (ext === "dockerfile" || name === "dockerfile") return "dockerfile";
  return "plaintext";
}

export function formatSourceSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  return `${(bytes / 1024).toFixed(1)} KB`;
}

export function countSourceFiles(entries: SourceEntry[]): number {
  return entries.reduce((count, entry) => {
    if (entry.kind === "file") return count + 1;
    return count + countSourceFiles(entry.children);
  }, 0);
}
