import Editor, { type Monaco, type OnMount } from "@monaco-editor/react";
import { useEffect, useRef } from "react";
import type { editor, IDisposable, Position } from "monaco-editor";
import type { BlameLine } from "../types";
import { setupMonaco } from "../monaco/setup";
import {
  fileHeaderSummary,
  formatRelative,
  inlineBlameText,
  isUncommitted,
  recencyBucket,
} from "./gitlens";

// Runs once on import, before the first <Editor> mounts (offline worker/loader).
setupMonaco();

function hoverMarkdown(line: BlameLine): string {
  const abs = line.date ? new Date(line.date).toLocaleString() : "";
  if (isUncommitted(line)) {
    const rel = formatRelative(line.date);
    return [`**Não commitado ainda**`, "", `${rel}${abs ? ` · ${abs}` : ""}`].join("\n");
  }
  return [
    `**${line.author}** ${line.author_email ? `<${line.author_email}>` : ""}`.trim(),
    "",
    line.summary,
    "",
    `\`${line.short_sha}\` · ${formatRelative(line.date)}${abs ? ` · ${abs}` : ""}`,
  ].join("\n");
}

export function MonacoSource({
  value,
  language,
  readOnly = false,
  onChange,
  onMount,
  onSave,
  onClose,
  onQuickOpen,
  onFindInFiles,
  onFormat,
  onCursorChange,
  blame,
  height = "100%",
  fontSize = 15,
  revealLine,
  path,
}: {
  value: string;
  language: string;
  readOnly?: boolean;
  onChange?: (content: string) => void;
  onMount?: OnMount;
  onSave?: () => void;
  onClose?: () => void;
  onQuickOpen?: () => void;
  onFindInFiles?: () => void;
  onFormat?: (text: string, language: string) => Promise<string | null>;
  onCursorChange?: (line: number, col: number) => void;
  blame?: BlameLine[];
  height?: string;
  fontSize?: number;
  revealLine?: number | null;
  path?: string;
}) {
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const monacoRef = useRef<Monaco | null>(null);
  const gutter = useRef<editor.IEditorDecorationsCollection | null>(null);
  const inline = useRef<editor.IEditorDecorationsCollection | null>(null);
  const blameRef = useRef<BlameLine[]>([]);
  const hoverProvider = useRef<IDisposable | null>(null);
  const codeLensProvider = useRef<IDisposable | null>(null);
  // Commands are registered once on mount; keep latest callbacks in refs to avoid stale closures.
  const onSaveRef = useRef(onSave);
  const onCloseRef = useRef(onClose);
  const onQuickOpenRef = useRef(onQuickOpen);
  const onFindInFilesRef = useRef(onFindInFiles);
  const onFormatRef = useRef(onFormat);
  const onCursorRef = useRef(onCursorChange);
  useEffect(() => {
    onSaveRef.current = onSave;
    onCloseRef.current = onClose;
    onQuickOpenRef.current = onQuickOpen;
    onFindInFilesRef.current = onFindInFiles;
    onFormatRef.current = onFormat;
    onCursorRef.current = onCursorChange;
  }, [onSave, onClose, onQuickOpen, onFindInFiles, onFormat, onCursorChange]);

  function updateInline(lineNumber: number) {
    const ed = editorRef.current;
    const monaco = monacoRef.current;
    if (!ed || !monaco || !inline.current) return;
    const model = ed.getModel();
    const found = blameRef.current.find((b) => b.line === lineNumber);
    if (!model || !found) {
      inline.current.set([]);
      return;
    }
    const col = model.getLineMaxColumn(lineNumber);
    inline.current.set([
      {
        range: new monaco.Range(lineNumber, col, lineNumber, col),
        options: {
          after: { content: `    ${inlineBlameText(found)}`, inlineClassName: "blame-inline" },
          showIfCollapsed: true,
        },
      },
    ]);
  }

  function applyGutter() {
    const ed = editorRef.current;
    const monaco = monacoRef.current;
    if (!ed || !monaco || !gutter.current) return;
    const model = ed.getModel();
    if (!model) return;
    gutter.current.set(
      blameRef.current.map((b) => {
        const bucket = isUncommitted(b) ? "uncommitted" : recencyBucket(b.date);
        return {
          range: new monaco.Range(b.line, 1, b.line, 1),
          options: {
            isWholeLine: true,
            linesDecorationsClassName: `blame-age blame-age-${bucket}`,
          },
        };
      }),
    );
  }

  const handleMount: OnMount = (ed, monaco) => {
    editorRef.current = ed;
    monacoRef.current = monaco;
    gutter.current = ed.createDecorationsCollection();
    inline.current = ed.createDecorationsCollection();
    ed.onDidChangeCursorPosition((event) => {
      updateInline(event.position.lineNumber);
      onCursorRef.current?.(event.position.lineNumber, event.position.column);
    });
    ed.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => onSaveRef.current?.());
    ed.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyW, () => onCloseRef.current?.());
    ed.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyP, () => onQuickOpenRef.current?.());
    ed.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyMod.Shift | monaco.KeyCode.KeyF, () =>
      onFindInFilesRef.current?.(),
    );
    ed.addCommand(monaco.KeyMod.Alt | monaco.KeyMod.Shift | monaco.KeyCode.KeyF, () => {
      const format = onFormatRef.current;
      const model = ed.getModel();
      if (!format || !model) return;
      const current = model.getValue();
      void format(current, model.getLanguageId()).then((formatted) => {
        if (formatted == null || formatted === model.getValue()) return;
        ed.executeEdits("format", [
          { range: model.getFullModelRange(), text: formatted, forceMoveMarkers: true },
        ]);
        ed.pushUndoStop();
      });
    });
    applyGutter();
    updateInline(ed.getPosition()?.lineNumber ?? 1);
    const start = ed.getPosition();
    if (start) onCursorRef.current?.(start.lineNumber, start.column);
    onMount?.(ed, monaco);
  };

  // Apply font-size changes live to the mounted editor.
  useEffect(() => {
    editorRef.current?.updateOptions({ fontSize });
  }, [fontSize]);

  // Jump to a line (from search results); re-runs when the target or file changes.
  useEffect(() => {
    const ed = editorRef.current;
    if (!ed || !revealLine || revealLine < 1) return;
    ed.revealLineInCenter(revealLine);
    ed.setPosition({ lineNumber: revealLine, column: 1 });
    ed.focus();
  }, [revealLine, value]);

  // Re-apply blame decorations whenever the blame data changes.
  useEffect(() => {
    blameRef.current = blame ?? [];
    applyGutter();
    updateInline(editorRef.current?.getPosition()?.lineNumber ?? 1);
  }, [blame]);

  // Register per-language hover + file-header CodeLens reading the current blame.
  useEffect(() => {
    const monaco = monacoRef.current;
    if (!monaco) return;
    hoverProvider.current?.dispose();
    hoverProvider.current = monaco.languages.registerHoverProvider(language, {
      provideHover(_model: editor.ITextModel, position: Position) {
        const found = blameRef.current.find((b) => b.line === position.lineNumber);
        if (!found) return null;
        return { contents: [{ value: hoverMarkdown(found) }] };
      },
    });

    codeLensProvider.current?.dispose();
    codeLensProvider.current = monaco.languages.registerCodeLensProvider(language, {
      provideCodeLenses(model: editor.ITextModel) {
        if (blameRef.current.length === 0) return { lenses: [], dispose() {} };
        return {
          lenses: [
            {
              range: new monaco.Range(1, 1, 1, 1),
              id: `blame-header-${model.id}`,
              command: { id: "", title: fileHeaderSummary(blameRef.current) },
            },
          ],
          dispose() {},
        };
      },
    });

    return () => {
      hoverProvider.current?.dispose();
      codeLensProvider.current?.dispose();
    };
  }, [language, blame]);

  return (
    <Editor
      value={value}
      language={language}
      path={path}
      theme="dw-dark"
      height={height}
      onChange={(next) => onChange?.(next ?? "")}
      onMount={handleMount}
      options={{
        readOnly,
        minimap: { enabled: false },
        fontSize,
        lineNumbers: "on",
        scrollBeyondLastLine: false,
        automaticLayout: true,
        renderLineHighlight: "line",
        smoothScrolling: true,
        tabSize: 2,
        glyphMargin: false,
        fontFamily: "'JetBrains Mono', 'SFMono-Regular', Consolas, monospace",
        scrollbar: { useShadows: false },
      }}
    />
  );
}
