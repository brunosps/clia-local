import { Boxes, FolderGit2, Plus, Check, Upload } from "lucide-react";
import { useMemo, useState } from "react";
import { createPortal } from "react-dom";
import type { Project, Workspace } from "./types";
import { projectDisplayName } from "./workspace";

type Row =
  | { kind: "workspace"; workspace: Workspace }
  | { kind: "project"; project: Project }
  | { kind: "add-workspace" }
  | { kind: "import-workspace" }
  | { kind: "add-project" };

/** Ctrl/Cmd+K palette to switch workspace/project by typing. */
export function QuickSwitch({
  workspaces,
  projects,
  activeWorkspaceId,
  activeProjectId,
  onPickWorkspace,
  onPickProject,
  onAddWorkspace,
  onImportWorkspace,
  onAddProject,
  onClose,
}: {
  workspaces: Workspace[];
  projects: Project[];
  activeWorkspaceId: number | null;
  activeProjectId: number | null;
  onPickWorkspace: (workspace: Workspace) => void;
  onPickProject: (project: Project) => void;
  onAddWorkspace: () => void;
  onImportWorkspace: () => void;
  onAddProject: () => void;
  onClose: () => void;
}) {
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);

  const rows = useMemo<Row[]>(() => {
    const q = query.trim().toLowerCase();
    const matchedWorkspaces = workspaces.filter((w) => w.name.toLowerCase().includes(q));
    const matchedProjects = projects.filter((p) => projectDisplayName(p).toLowerCase().includes(q));
    return [
      ...matchedWorkspaces.map((workspace): Row => ({ kind: "workspace", workspace })),
      ...matchedProjects.map((project): Row => ({ kind: "project", project })),
      { kind: "add-workspace" },
      { kind: "import-workspace" },
      { kind: "add-project" },
    ];
  }, [query, workspaces, projects]);

  // Clamp at render so a shrinking filtered list never points past the end.
  const activeIndex = rows.length ? Math.min(selected, rows.length - 1) : 0;

  function activate(row: Row) {
    if (row.kind === "workspace") onPickWorkspace(row.workspace);
    else if (row.kind === "project") onPickProject(row.project);
    else if (row.kind === "add-workspace") onAddWorkspace();
    else if (row.kind === "import-workspace") onImportWorkspace();
    else onAddProject();
  }

  function onKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setSelected((current) => (current + 1) % rows.length);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setSelected((current) => (current - 1 + rows.length) % rows.length);
    } else if (event.key === "Enter") {
      event.preventDefault();
      const row = rows[activeIndex];
      if (row) activate(row);
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
        aria-label="Trocar contexto"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <input
          autoFocus
          className="quick-switch-input"
          placeholder="Buscar workspace ou projeto…"
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
            setSelected(0);
          }}
          onKeyDown={onKeyDown}
          aria-label="Buscar workspace ou projeto"
        />
        <div className="quick-switch-list">
          {rows.map((row, index) => {
            const active = index === activeIndex;
            const className = active ? "quick-switch-row active" : "quick-switch-row";
            if (row.kind === "workspace") {
              return (
                <button
                  key={`ws-${row.workspace.id}`}
                  type="button"
                  className={className}
                  onMouseEnter={() => setSelected(index)}
                  onClick={() => activate(row)}
                >
                  <Boxes aria-hidden="true" size={15} />
                  <span className="quick-switch-label">{row.workspace.name}</span>
                  <span className="quick-switch-kind">workspace</span>
                  {row.workspace.id === activeWorkspaceId ? (
                    <Check aria-hidden="true" size={14} />
                  ) : null}
                </button>
              );
            }
            if (row.kind === "project") {
              const submodule = Boolean(row.project.is_submodule);
              return (
                <button
                  key={`proj-${row.project.id}`}
                  type="button"
                  className={submodule ? `${className} is-submodule` : className}
                  onMouseEnter={() => setSelected(index)}
                  onClick={() => activate(row)}
                >
                  <FolderGit2 aria-hidden="true" size={15} />
                  <span className="quick-switch-label">{projectDisplayName(row.project)}</span>
                  <span className="quick-switch-kind">{submodule ? "submodule" : "projeto"}</span>
                  {row.project.id === activeProjectId ? (
                    <Check aria-hidden="true" size={14} />
                  ) : null}
                </button>
              );
            }
            return (
              <button
                key={row.kind}
                type="button"
                className={className}
                onMouseEnter={() => setSelected(index)}
                onClick={() => activate(row)}
              >
                {row.kind === "import-workspace" ? (
                  <Upload aria-hidden="true" size={15} />
                ) : (
                  <Plus aria-hidden="true" size={15} />
                )}
                <span className="quick-switch-label">
                  {row.kind === "add-workspace"
                    ? "Novo workspace"
                    : row.kind === "import-workspace"
                      ? "Importar .wksdw"
                      : "Adicionar projeto"}
                </span>
              </button>
            );
          })}
        </div>
      </section>
    </div>,
    document.body,
  );
}
