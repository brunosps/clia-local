import * as prettier from "prettier/standalone";
import babel from "prettier/plugins/babel";
import estree from "prettier/plugins/estree";
import typescript from "prettier/plugins/typescript";
import postcss from "prettier/plugins/postcss";
import html from "prettier/plugins/html";
import markdown from "prettier/plugins/markdown";
import yaml from "prettier/plugins/yaml";
import type { Plugin } from "prettier";

const PRETTIER_PLUGINS: Plugin[] = [babel, estree, typescript, postcss, html, markdown, yaml];

/** Prettier parser for a Monaco language id, or null if Prettier doesn't cover it. */
export function prettierParser(language: string): string | null {
  switch (language) {
    case "typescript":
      return "typescript";
    case "javascript":
      return "babel";
    case "json":
      return "json";
    case "css":
      return "css";
    case "scss":
      return "scss";
    case "less":
      return "less";
    case "html":
      return "html";
    case "markdown":
      return "markdown";
    case "yaml":
      return "yaml";
    default:
      return null;
  }
}

/** Languages formatted by an external CLI on the Rust side. */
export function externalFormatterLanguage(language: string): boolean {
  return language === "rust" || language === "python" || language === "csharp";
}

export function formatWithPrettier(text: string, parser: string): Promise<string> {
  return prettier.format(text, { parser, plugins: PRETTIER_PLUGINS });
}
