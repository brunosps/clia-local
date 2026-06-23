import { DiffEditor } from "@monaco-editor/react";
import { X } from "lucide-react";
import { useEffect } from "react";
import { setupMonaco } from "../monaco/setup";

setupMonaco();

/** Side-by-side comparison of a historical file version against the working copy. */
export function DiffCompare({
  path,
  leftLabel,
  rightLabel,
  original,
  modified,
  language,
  fontSize = 15,
  onClose,
}: {
  path: string;
  leftLabel: string;
  rightLabel: string;
  original: string;
  modified: string;
  language: string;
  fontSize?: number;
  onClose: () => void;
}) {
  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div className="source-overlay" role="dialog" aria-modal="true" aria-label="Comparar versões">
      <div className="source-overlay-card wide">
        <div className="source-overlay-head">
          <span>
            Comparando <code>{leftLabel}</code> ↔ <code>{rightLabel}</code> · {path.split("/").pop()}
          </span>
          <button className="secondary-button icon-button" type="button" onClick={onClose} aria-label="Fechar">
            <X aria-hidden="true" size={14} />
          </button>
        </div>
        <div className="source-overlay-body">
          <DiffEditor
            theme="dw-dark"
            language={language}
            original={original}
            modified={modified}
            height="100%"
            options={{
              readOnly: true,
              renderSideBySide: true,
              minimap: { enabled: false },
              fontFamily: "'JetBrains Mono', monospace",
              fontSize,
              scrollBeyondLastLine: false,
            }}
          />
        </div>
      </div>
    </div>
  );
}
