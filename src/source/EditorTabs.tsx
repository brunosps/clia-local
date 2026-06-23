import { Eye, X } from "lucide-react";
import type { SourceFile } from "../types";
import { fileIcon } from "./fileIcons";

function baseName(path: string) {
  return path.split("/").pop() ?? path;
}

function isMarkdown(path: string) {
  const ext = path.split(".").pop()?.toLowerCase();
  return ext === "md" || ext === "mdx";
}

export function EditorTabs({
  openFiles,
  activePath,
  dirty,
  previewActive,
  onSelect,
  onClose,
  onTogglePreview,
}: {
  openFiles: SourceFile[];
  activePath: string | null;
  dirty: boolean;
  previewActive: boolean;
  onSelect: (relativePath: string) => void;
  onClose: (relativePath: string) => void;
  onTogglePreview: (relativePath: string) => void;
}) {
  if (!openFiles.length) return null;
  return (
    <div className="editor-tabs" role="tablist">
      {openFiles.map((file) => {
        const { Icon, color } = fileIcon(file.relative_path);
        const active = file.relative_path === activePath;
        const markdown = isMarkdown(file.relative_path);
        return (
          <div
            key={file.relative_path}
            className={active ? "editor-tab active" : "editor-tab"}
            role="tab"
            aria-selected={active}
            // Middle-click closes the tab (VSCode behavior).
            onAuxClick={(event) => {
              if (event.button === 1) {
                event.preventDefault();
                onClose(file.relative_path);
              }
            }}
          >
            <button
              type="button"
              className="editor-tab-open"
              onClick={() => onSelect(file.relative_path)}
              title={file.relative_path}
            >
              <Icon aria-hidden="true" size={15} style={{ color }} />
              <span>{baseName(file.relative_path)}</span>
              {active && dirty ? <span className="editor-tab-dirty" aria-label="não salvo" /> : null}
            </button>
            {markdown ? (
              <button
                type="button"
                className={
                  active && previewActive ? "editor-tab-eye active" : "editor-tab-eye"
                }
                aria-label="Visualizar markdown"
                title="Visualizar markdown"
                aria-pressed={active && previewActive}
                onClick={() => onTogglePreview(file.relative_path)}
              >
                <Eye aria-hidden="true" size={14} />
              </button>
            ) : null}
            <button
              type="button"
              className="editor-tab-close"
              aria-label={`Fechar ${baseName(file.relative_path)}`}
              onClick={() => onClose(file.relative_path)}
            >
              <X aria-hidden="true" size={14} />
            </button>
          </div>
        );
      })}
    </div>
  );
}

export function Breadcrumb({ path }: { path: string | null }) {
  if (!path) return null;
  const segments = path.split("/").filter(Boolean);
  return (
    <div className="editor-breadcrumb" aria-label="Caminho do arquivo">
      {segments.map((segment, index) => (
        <span key={index} className="editor-breadcrumb-seg">
          {index > 0 ? <span className="editor-breadcrumb-sep">›</span> : null}
          {segment}
        </span>
      ))}
    </div>
  );
}
