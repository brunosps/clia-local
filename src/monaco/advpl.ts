// AdvPL / TLPP syntax highlighting for Monaco, driven by the official TOTVS
// TextMate grammar (vendored from totvs/tds-vscode, Apache-2.0 — see ./advpl/NOTICE.md).
//
// Monaco has no built-in TextMate support, so we bridge it: the grammar is run
// through vscode-textmate (with the vscode-oniguruma WASM regex engine) and each
// line's TextMate scopes are mapped to coarse Monaco token kinds so the existing
// "dw-dark" theme colors them. Tokenization parity comes from the real grammar;
// colors come from the scope→token mapping below.
import * as monaco from "monaco-editor";
import * as oniguruma from "vscode-oniguruma";
import * as vsctm from "vscode-textmate";
import onigWasmUrl from "vscode-oniguruma/release/onig.wasm?url";
import advplGrammarRaw from "./advpl/advpl.tmLanguage.json?raw";

const ADVPL_LANGUAGE_ID = "advpl";
const ADVPL_SCOPE = "source.advpl";

// Extensions handled by the AdvPL grammar (PRW/TLPP and friends).
const ADVPL_EXTENSIONS = [
  ".prw",
  ".prx",
  ".prg",
  ".ppx",
  ".ppp",
  ".tlpp",
  ".ch",
  ".th",
  ".ahu",
  ".apl",
  ".apw",
];

/** Wraps a vscode-textmate rule stack as a Monaco tokenizer state (immutable → ref-equality). */
class TextMateState implements monaco.languages.IState {
  constructor(readonly ruleStack: vsctm.StateStack) {}
  clone(): monaco.languages.IState {
    return new TextMateState(this.ruleStack);
  }
  equals(other: monaco.languages.IState): boolean {
    return other instanceof TextMateState && other.ruleStack === this.ruleStack;
  }
}

/** Map the most specific TextMate scope of a token to a Monaco token kind the theme colors. */
function tmScopeToMonacoToken(scopes: string[]): string {
  const scope = scopes[scopes.length - 1] ?? "";
  if (scope.startsWith("comment")) return "comment";
  if (scope.startsWith("punctuation.definition.comment")) return "comment";
  if (scope.startsWith("constant.numeric")) return "number";
  if (scope.startsWith("constant.character")) return "string";
  if (scope.startsWith("string")) return "string";
  if (scope.startsWith("constant.language")) return "keyword";
  if (scope.startsWith("constant")) return "constant";
  if (scope.startsWith("keyword.operator")) return "operator";
  if (scope.startsWith("keyword")) return "keyword";
  if (scope.startsWith("storage")) return "keyword";
  if (scope.startsWith("variable.language")) return "keyword";
  if (scope.startsWith("support.function")) return "type";
  if (scope.startsWith("support")) return "type";
  if (scope.startsWith("entity.name")) return "type";
  if (scope.startsWith("meta.preprocessor")) return "keyword";
  if (scope.startsWith("variable")) return "variable";
  return "";
}

const ADVPL_CONFIGURATION: monaco.languages.LanguageConfiguration = {
  comments: { lineComment: "//", blockComment: ["/*", "*/"] },
  brackets: [
    ["{", "}"],
    ["[", "]"],
    ["(", ")"],
  ],
  autoClosingPairs: [
    { open: "{", close: "}" },
    { open: "[", close: "]" },
    { open: "(", close: ")" },
    { open: '"', close: '"' },
    { open: "'", close: "'" },
  ],
  surroundingPairs: [
    { open: "{", close: "}" },
    { open: "[", close: "]" },
    { open: "(", close: ")" },
    { open: '"', close: '"' },
    { open: "'", close: "'" },
  ],
};

let onigReady: Promise<void> | null = null;
function ensureOniguruma(): Promise<void> {
  // loadWASM may only run once per process.
  if (!onigReady) {
    onigReady = fetch(onigWasmUrl)
      .then((response) => oniguruma.loadWASM(response))
      .then(() => undefined);
  }
  return onigReady;
}

let registered = false;

/**
 * Register the `advpl` language and attach the TextMate-backed tokenizer. The
 * language id + configuration are registered synchronously (so models can open
 * as `advpl` immediately); the grammar loads asynchronously and Monaco re-tokenizes
 * once the tokens provider is set. Idempotent and safe to fire-and-forget.
 */
export async function registerAdvpl(): Promise<void> {
  if (registered) return;
  registered = true;

  monaco.languages.register({
    id: ADVPL_LANGUAGE_ID,
    extensions: ADVPL_EXTENSIONS,
    aliases: ["AdvPL", "TLPP", "advpl"],
  });
  monaco.languages.setLanguageConfiguration(ADVPL_LANGUAGE_ID, ADVPL_CONFIGURATION);

  try {
    await ensureOniguruma();
    const registry = new vsctm.Registry({
      onigLib: Promise.resolve({
        createOnigScanner: (sources) => new oniguruma.OnigScanner(sources),
        createOnigString: (str) => new oniguruma.OnigString(str),
      }),
      loadGrammar: async (scopeName) =>
        scopeName === ADVPL_SCOPE
          ? vsctm.parseRawGrammar(advplGrammarRaw, "advpl.tmLanguage.json")
          : null,
    });

    const grammar = await registry.loadGrammar(ADVPL_SCOPE);
    if (!grammar) return;

    monaco.languages.setTokensProvider(ADVPL_LANGUAGE_ID, {
      getInitialState: () => new TextMateState(vsctm.INITIAL),
      tokenize: (line, state) => {
        const result = grammar.tokenizeLine(line, (state as TextMateState).ruleStack);
        return {
          tokens: result.tokens.map((token) => ({
            startIndex: token.startIndex,
            scopes: tmScopeToMonacoToken(token.scopes),
          })),
          endState: new TextMateState(result.ruleStack),
        };
      },
    });
  } catch (error) {
    // Highlighting is best-effort: if the WASM/grammar fails to load, AdvPL files
    // still open (as registered plaintext-ish) rather than breaking the editor.
    console.error("Failed to load AdvPL TextMate grammar", error);
  }
}
