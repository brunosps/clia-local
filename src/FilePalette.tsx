import { useMemo, useState } from "react";
import { createPortal } from "react-dom";
import type { SourceEntry, SourceFile } from "./types";
import { searchSourceFiles, type FileHit } from "./fileSearch";
import { fileIcon } from "./source/fileIcons";

function parentDir(relativePath: string): string {
  const slash = relativePath.lastIndexOf("/");
  return slash === -1 ? "" : relativePath.slice(0, slash);
}

/** VSCode-style Ctrl/Cmd+P "Go to File" palette. */
export function FilePalette({
  entries,
  openFiles,
  onOpen,
  onClose,
}: {
  entries: SourceEntry[];
  openFiles: SourceFile[];
  onOpen: (relativePath: string) => void;
  onClose: () => void;
}) {
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);

  const { results, recentPaths } = useMemo(() => {
    if (query.trim()) {
      return { results: searchSourceFiles(entries, query), recentPaths: new Set<string>() };
    }
    const recent: FileHit[] = openFiles.map((f) => ({ relative_path: f.relative_path, name: f.name }));
    const recentSet = new Set(recent.map((r) => r.relative_path));
    const rest = searchSourceFiles(entries, "").filter((f) => !recentSet.has(f.relative_path));
    return { results: [...recent, ...rest].slice(0, 50), recentPaths: recentSet };
  }, [query, entries, openFiles]);

  const activeIndex = results.length ? Math.min(selected, results.length - 1) : 0;

  function onKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setSelected((current) => (current + 1) % results.length);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setSelected((current) => (current - 1 + results.length) % results.length);
    } else if (event.key === "Enter") {
      event.preventDefault();
      const hit = results[activeIndex];
      if (hit) onOpen(hit.relative_path);
    } else if (event.key === "Escape") {
      event.preventDefault();
      onClose();
    }
  }

  return createPortal(
    <div className="modal-backdrop elevated" role="presentation" onMouseDown={onClose}>
      <section
        className="modal-panel quick-switch"
        role="dialog"
        aria-modal="true"
        aria-label="Abrir arquivo"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <input
          autoFocus
          className="quick-switch-input"
          placeholder="Buscar arquivo pelo nome…"
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
            setSelected(0);
          }}
          onKeyDown={onKeyDown}
          aria-label="Buscar arquivo pelo nome"
        />
        <div className="quick-switch-list">
          {results.length ? (
            results.map((hit, index) => {
              const { Icon, color } = fileIcon(hit.name);
              const dir = parentDir(hit.relative_path);
              return (
                <button
                  key={hit.relative_path}
                  type="button"
                  className={index === activeIndex ? "quick-switch-row active" : "quick-switch-row"}
                  onMouseEnter={() => setSelected(index)}
                  onClick={() => onOpen(hit.relative_path)}
                >
                  <Icon aria-hidden="true" size={15} style={{ color }} />
                  <span className="quick-switch-label">
                    <strong>{hit.name}</strong>
                    {dir ? <span className="file-palette-dir"> {dir}</span> : null}
                  </span>
                  {recentPaths.has(hit.relative_path) ? (
                    <span className="quick-switch-kind">recente</span>
                  ) : null}
                </button>
              );
            })
          ) : (
            <div className="empty-note">Nenhum arquivo encontrado.</div>
          )}
        </div>
      </section>
    </div>,
    document.body,
  );
}
