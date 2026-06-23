// Offline Monaco wiring for Tauri (no CDN). Imported once before the first
// <Editor> mounts so @monaco-editor/react uses the locally-bundled engine and
// Vite-emitted web workers instead of fetching from the network.
import * as monaco from "monaco-editor";
import { loader } from "@monaco-editor/react";
import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import jsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";
import cssWorker from "monaco-editor/esm/vs/language/css/css.worker?worker";
import htmlWorker from "monaco-editor/esm/vs/language/html/html.worker?worker";
import tsWorker from "monaco-editor/esm/vs/language/typescript/ts.worker?worker";

declare global {
  interface Window {
    MonacoEnvironment?: monaco.Environment;
  }
}

let configured = false;

/** Configure Monaco workers + the React loader + the dark theme. Idempotent. */
export function setupMonaco() {
  if (configured) return;
  configured = true;

  self.MonacoEnvironment = {
    getWorker(_workerId: string, label: string) {
      if (label === "json") return new jsonWorker();
      if (label === "css" || label === "scss" || label === "less") return new cssWorker();
      if (label === "html" || label === "handlebars" || label === "razor") return new htmlWorker();
      if (label === "typescript" || label === "javascript") return new tsWorker();
      return new editorWorker();
    },
  };

  monaco.editor.defineTheme("dw-dark", {
    base: "vs-dark",
    inherit: true,
    rules: [],
    colors: {
      "editor.background": "#0f1218",
      "editor.foreground": "#e8edf4",
      "editorLineNumber.foreground": "#57637a",
      "editorLineNumber.activeForeground": "#93a4b8",
      "editor.lineHighlightBackground": "#161b22",
      "editorGutter.background": "#0f1218",
      "editorIndentGuide.background1": "#1d232e",
    },
  });

  // Enable Monaco's built-in TypeScript/JavaScript language service (the same
  // engine VSCode uses for TS/JS): completion, hover types, signature help, and
  // syntax validation. Semantic validation is left off because we open one file
  // at a time without the project graph — it would flag false "cannot find
  // module" errors. Full cross-file diagnostics come from external servers (LSP).
  configureTypeScript();

  // Point the React wrapper at the local engine (avoids the default CDN fetch).
  loader.config({ monaco });
}

// The top-level `monaco-editor` types stub `languages.typescript` as deprecated,
// but the API is registered at runtime by the full import. Declare the slice we use.
interface TsServiceDefaults {
  setCompilerOptions(options: Record<string, unknown>): void;
  setDiagnosticsOptions(options: Record<string, unknown>): void;
  setEagerModelSync(value: boolean): void;
}
interface TsNamespace {
  typescriptDefaults: TsServiceDefaults;
  javascriptDefaults: TsServiceDefaults;
  ScriptTarget: { ESNext: number };
  ModuleKind: { ESNext: number };
  ModuleResolutionKind: { NodeJs: number };
  JsxEmit: { React: number };
}

function configureTypeScript() {
  const ts = monaco.languages.typescript as unknown as TsNamespace | undefined;
  if (!ts?.typescriptDefaults) return;

  const compilerOptions: Record<string, unknown> = {
    target: ts.ScriptTarget.ESNext,
    module: ts.ModuleKind.ESNext,
    moduleResolution: ts.ModuleResolutionKind.NodeJs,
    jsx: ts.JsxEmit.React,
    allowJs: true,
    allowNonTsExtensions: true,
    esModuleInterop: true,
    skipLibCheck: true,
    lib: ["esnext", "dom", "dom.iterable"],
  };
  const diagnosticsOptions: Record<string, unknown> = {
    noSemanticValidation: true,
    noSyntaxValidation: false,
    noSuggestionDiagnostics: true,
  };

  for (const defaults of [ts.typescriptDefaults, ts.javascriptDefaults]) {
    defaults.setCompilerOptions(compilerOptions);
    defaults.setDiagnosticsOptions(diagnosticsOptions);
    defaults.setEagerModelSync(true);
  }
}
