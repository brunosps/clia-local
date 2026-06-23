import { Check, GitCompareArrows, History, X } from "lucide-react";
import type { Commit } from "../types";
import { formatRelative } from "./gitlens";

const WORKING = "WORKING";

function compareLabel(id: string): string {
  return id === WORKING ? "Cópia de trabalho" : id.slice(0, 7);
}

export function FileHistoryPanel({
  commits,
  filePath,
  compareBase,
  onSelect,
  onPickCompare,
  onClose,
}: {
  commits: Commit[];
  filePath: string;
  compareBase: string | null;
  onSelect: (sha: string) => void;
  onPickCompare: (id: string) => void;
  onClose: () => void;
}) {
  function compareTitle(id: string): string {
    if (compareBase === null) return "Marcar para comparar";
    if (compareBase === id) return "Cancelar seleção";
    return `Comparar com ${compareLabel(compareBase)}`;
  }

  return (
    <aside className="file-history" aria-label="Histórico do arquivo">
      <div className="file-history-head">
        <span>
          <History aria-hidden="true" size={14} /> Histórico · {filePath.split("/").pop()}
        </span>
        <button className="secondary-button icon-button" type="button" onClick={onClose} aria-label="Fechar">
          <X aria-hidden="true" size={14} />
        </button>
      </div>

      {compareBase !== null ? (
        <div className="file-history-banner">
          <span>
            Comparando a partir de <strong>{compareLabel(compareBase)}</strong> — escolha a segunda versão
          </span>
          <button type="button" className="ghost-button" onClick={() => onPickCompare(compareBase)}>
            Cancelar
          </button>
        </div>
      ) : null}

      <div className="file-history-list">
        <div className={compareBase === WORKING ? "file-history-row working active" : "file-history-row working"}>
          <span className="file-history-open static">
            <strong>Cópia de trabalho (atual)</strong>
            <small>versão não commitada no disco</small>
          </span>
          <button
            type="button"
            className={compareBase === WORKING ? "secondary-button icon-button active" : "secondary-button icon-button"}
            title={compareTitle(WORKING)}
            onClick={() => onPickCompare(WORKING)}
          >
            {compareBase === WORKING ? (
              <Check aria-hidden="true" size={14} />
            ) : (
              <GitCompareArrows aria-hidden="true" size={14} />
            )}
          </button>
        </div>

        {commits.length ? (
          commits.map((commit) => (
            <div
              key={commit.sha}
              className={commit.sha === compareBase ? "file-history-row active" : "file-history-row"}
            >
              <button
                type="button"
                className="file-history-open"
                onClick={() => onSelect(commit.sha)}
                title="Ver esta versão"
              >
                <strong>{commit.subject}</strong>
                <small>
                  {commit.short_sha} · {commit.author_name} · {formatRelative(commit.date)}
                </small>
              </button>
              <button
                type="button"
                className={
                  commit.sha === compareBase ? "secondary-button icon-button active" : "secondary-button icon-button"
                }
                title={compareTitle(commit.sha)}
                onClick={() => onPickCompare(commit.sha)}
              >
                {commit.sha === compareBase ? (
                  <Check aria-hidden="true" size={14} />
                ) : (
                  <GitCompareArrows aria-hidden="true" size={14} />
                )}
              </button>
            </div>
          ))
        ) : (
          <div className="empty-note">Sem histórico para este arquivo.</div>
        )}
      </div>
    </aside>
  );
}
