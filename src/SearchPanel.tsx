import { CaseSensitive, ChevronDown, ChevronRight, Regex, WholeWord } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { api } from "./tauri";
import type { SearchFileResult } from "./types";
import { fileIcon } from "./source/fileIcons";

function parentDir(relativePath: string): string {
  const slash = relativePath.lastIndexOf("/");
  return slash === -1 ? "" : relativePath.slice(0, slash);
}

function baseName(relativePath: string): string {
  return relativePath.split("/").pop() ?? relativePath;
}

/** Highlight the matched span [col, col+length) within a result line. */
function MatchText({ text, col, length }: { text: string; col: number; length: number }) {
  const chars = [...text];
  const before = chars.slice(0, col).join("").trimStart();
  const hit = chars.slice(col, col + length).join("");
  const after = chars.slice(col + length).join("");
  return (
    <span className="search-line">
      {before}
      <mark>{hit}</mark>
      {after}
    </span>
  );
}

/** VSCode-style "Find in Files" panel (the Search tab of the source sidebar). */
export function SearchPanel({
  path,
  focusSeed,
  onOpenResult,
}: {
  path: string;
  focusSeed: number;
  onOpenResult: (relativePath: string, line: number) => void;
}) {
  const [query, setQuery] = useState("");
  const [caseSensitive, setCaseSensitive] = useState(false);
  const [wholeWord, setWholeWord] = useState(false);
  const [useRegex, setUseRegex] = useState(false);
  const [results, setResults] = useState<SearchFileResult[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const inputRef = useRef<HTMLInputElement | null>(null);

  // Focus the search field when the Search tab is (re)opened.
  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, [focusSeed]);

  // Debounced search whenever the query or any toggle changes.
  useEffect(() => {
    const handle = window.setTimeout(() => {
      if (!query.trim() || !path) {
        setResults([]);
        setError("");
        setBusy(false);
        return;
      }
      setBusy(true);
      void api
        .searchInFiles(path, query, { caseSensitive, wholeWord, useRegex })
        .then((result) => {
          if (result.ok) {
            setResults(result.value);
            setError("");
          } else {
            setResults([]);
            setError(result.error);
          }
          setBusy(false);
        });
    }, 250);
    return () => window.clearTimeout(handle);
  }, [query, caseSensitive, wholeWord, useRegex, path]);

  function toggleFile(relativePath: string) {
    setCollapsed((current) => {
      const next = new Set(current);
      if (next.has(relativePath)) next.delete(relativePath);
      else next.add(relativePath);
      return next;
    });
  }

  const totalMatches = results.reduce((sum, file) => sum + file.matches.length, 0);

  return (
    <div className="search-panel">
      <div className="search-input-row">
        <input
          ref={inputRef}
          className="search-input"
          placeholder="Buscar"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          aria-label="Buscar nos arquivos"
        />
        <div className="search-toggles">
          <button
            type="button"
            className={caseSensitive ? "search-toggle active" : "search-toggle"}
            onClick={() => setCaseSensitive((value) => !value)}
            title="Diferenciar maiúsculas/minúsculas"
            aria-pressed={caseSensitive}
          >
            <CaseSensitive aria-hidden="true" size={15} />
          </button>
          <button
            type="button"
            className={wholeWord ? "search-toggle active" : "search-toggle"}
            onClick={() => setWholeWord((value) => !value)}
            title="Palavra inteira"
            aria-pressed={wholeWord}
          >
            <WholeWord aria-hidden="true" size={15} />
          </button>
          <button
            type="button"
            className={useRegex ? "search-toggle active" : "search-toggle"}
            onClick={() => setUseRegex((value) => !value)}
            title="Usar expressão regular"
            aria-pressed={useRegex}
          >
            <Regex aria-hidden="true" size={15} />
          </button>
        </div>
      </div>

      <div className="search-summary">
        {error ? (
          <span className="search-error">{error}</span>
        ) : busy ? (
          <span>Buscando…</span>
        ) : query.trim() ? (
          <span>
            {totalMatches} resultado(s) em {results.length} arquivo(s)
          </span>
        ) : (
          <span>Digite para buscar no projeto.</span>
        )}
      </div>

      <div className="search-results">
        {results.map((file) => {
          const isCollapsed = collapsed.has(file.relative_path);
          const { Icon, color } = fileIcon(baseName(file.relative_path));
          const dir = parentDir(file.relative_path);
          return (
            <div className="search-file" key={file.relative_path}>
              <button
                type="button"
                className="search-file-head"
                onClick={() => toggleFile(file.relative_path)}
              >
                {isCollapsed ? (
                  <ChevronRight aria-hidden="true" size={14} />
                ) : (
                  <ChevronDown aria-hidden="true" size={14} />
                )}
                <Icon aria-hidden="true" size={14} style={{ color }} />
                <span className="search-file-name">{baseName(file.relative_path)}</span>
                {dir ? <span className="search-file-dir">{dir}</span> : null}
                <span className="search-file-count">{file.matches.length}</span>
              </button>
              {isCollapsed
                ? null
                : file.matches.map((match, index) => (
                    <button
                      type="button"
                      className="search-match"
                      key={`${match.line}-${match.col}-${index}`}
                      onClick={() => onOpenResult(file.relative_path, match.line)}
                      title={`Linha ${match.line}`}
                    >
                      <span className="search-match-line">{match.line}</span>
                      <MatchText text={match.text} col={match.col} length={match.length} />
                    </button>
                  ))}
            </div>
          );
        })}
      </div>
    </div>
  );
}
