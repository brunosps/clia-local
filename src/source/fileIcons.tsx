import {
  Braces,
  File,
  FileCode,
  FileText,
  FileType,
  Hash,
  Image,
  Settings2,
  type LucideIcon,
} from "lucide-react";

/** Pick a colored lucide icon for a file name (VSCode-ish icon theme). */
export function fileIcon(name: string): { Icon: LucideIcon; color: string } {
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  switch (ext) {
    case "ts":
    case "tsx":
    case "mts":
    case "cts":
      return { Icon: FileCode, color: "#3178c6" };
    case "js":
    case "jsx":
    case "mjs":
    case "cjs":
      return { Icon: FileCode, color: "#e8d44d" };
    case "rs":
      return { Icon: FileCode, color: "#dea584" };
    case "py":
      return { Icon: FileCode, color: "#5a9fd4" };
    case "go":
      return { Icon: FileCode, color: "#7fd1e8" };
    case "json":
      return { Icon: Braces, color: "#cbcb41" };
    case "css":
    case "scss":
    case "less":
      return { Icon: FileType, color: "#a07cff" };
    case "html":
    case "htm":
    case "xml":
    case "svg":
      return { Icon: FileType, color: "#e08a5a" };
    case "md":
    case "mdx":
      return { Icon: FileText, color: "#9ad27f" };
    case "yml":
    case "yaml":
    case "toml":
      return { Icon: Settings2, color: "#6ea8fe" };
    case "png":
    case "jpg":
    case "jpeg":
    case "gif":
    case "webp":
      return { Icon: Image, color: "#b99cff" };
    case "sh":
    case "bash":
    case "zsh":
      return { Icon: Hash, color: "#89e051" };
    default:
      return { Icon: File, color: "#93a4b8" };
  }
}
