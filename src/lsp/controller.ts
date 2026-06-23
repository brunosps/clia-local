import * as monaco from "monaco-editor";
import { listen } from "@tauri-apps/api/event";
import { api } from "../tauri";

/** Language servers we know how to launch (must match the Rust allowlist). */
type ServerLang = "rust" | "typescript";

function serverLangFor(language: string): ServerLang | null {
  if (language === "rust") return "rust";
  if (language === "typescript" || language === "javascript") return "typescript";
  return null;
}

/** Absolute filesystem path → file URI that matches the Monaco model URI. */
export function fileUriFor(absolutePath: string): string {
  return monaco.Uri.file(absolutePath).toString();
}

interface JsonRpc {
  jsonrpc?: string;
  id?: number | string;
  method?: string;
  params?: unknown;
  result?: unknown;
}

interface ServerState {
  key: string;
  serverLang: ServerLang;
  projectPath: string;
  id: number | null;
  initialized: boolean;
  available: boolean;
  initializeId: number;
  nextId: number;
  starting: Promise<void> | null;
  queue: JsonRpc[];
}

interface DocState {
  serverKey: string;
  version: number;
}

const LSP_SEVERITY: Record<number, number> = {
  1: monaco.MarkerSeverity.Error,
  2: monaco.MarkerSeverity.Warning,
  3: monaco.MarkerSeverity.Info,
  4: monaco.MarkerSeverity.Hint,
};

class LspController {
  private servers = new Map<string, ServerState>();
  private docs = new Map<string, DocState>();
  private listening = false;
  private enabled = false;
  private onError: ((language: string, message: string) => void) | null = null;

  setErrorHandler(handler: (language: string, message: string) => void) {
    this.onError = handler;
  }

  setEnabled(enabled: boolean) {
    this.enabled = enabled;
    if (!enabled) this.reset();
  }

  isEnabled() {
    return this.enabled;
  }

  /** Languages this controller can drive a server for. */
  supports(language: string) {
    return serverLangFor(language) !== null;
  }

  private async ensureListener() {
    if (this.listening) return;
    this.listening = true;
    await listen<{ server_id: number; message: string }>("lsp://message", (event) => {
      this.handleMessage(event.payload.server_id, event.payload.message);
    });
  }

  private serverById(id: number): ServerState | undefined {
    for (const server of this.servers.values()) if (server.id === id) return server;
    return undefined;
  }

  private rawSend(server: ServerState, message: JsonRpc) {
    if (server.id === null) return;
    void api.lsp_send(server.id, JSON.stringify(message));
  }

  /** Send a client→server notification, queued until the server is initialized. */
  private notify(server: ServerState, method: string, params: unknown) {
    const message: JsonRpc = { jsonrpc: "2.0", method, params };
    if (!server.initialized) server.queue.push(message);
    else this.rawSend(server, message);
  }

  private async ensureServer(serverLang: ServerLang, projectPath: string): Promise<ServerState | null> {
    await this.ensureListener();
    const key = `${projectPath}::${serverLang}`;
    let server = this.servers.get(key);
    if (server) {
      if (server.starting) await server.starting;
      return server.available ? server : null;
    }
    server = {
      key,
      serverLang,
      projectPath,
      id: null,
      initialized: false,
      available: true,
      initializeId: 1,
      nextId: 2,
      starting: null,
      queue: [],
    };
    this.servers.set(key, server);
    const current = server;
    server.starting = (async () => {
      const started = await api.lsp_start(serverLang, projectPath);
      if (!started.ok) {
        current.available = false;
        // A missing binary is surfaced by the footer install chip, not the banner.
        if (!started.error.toLowerCase().includes("not found")) {
          this.onError?.(serverLang, started.error);
        }
        return;
      }
      current.id = started.value;
      this.rawSend(current, {
        jsonrpc: "2.0",
        id: current.initializeId,
        method: "initialize",
        params: {
          processId: null,
          clientInfo: { name: "clia-app" },
          rootUri: fileUriFor(projectPath),
          workspaceFolders: [{ uri: fileUriFor(projectPath), name: "root" }],
          capabilities: {
            textDocument: {
              synchronization: { dynamicRegistration: false, didSave: false },
              publishDiagnostics: { relatedInformation: true },
            },
            workspace: { configuration: true, workspaceFolders: true },
          },
        },
      });
    })();
    await server.starting;
    server.starting = null;
    return server.available ? server : null;
  }

  private handleMessage(serverId: number, raw: string) {
    let message: JsonRpc;
    try {
      message = JSON.parse(raw) as JsonRpc;
    } catch {
      return;
    }
    const server = this.serverById(serverId);
    if (!server) return;

    // Response to our initialize → finish the handshake and flush queued notifies.
    if (message.id === server.initializeId && "result" in message) {
      server.initialized = true;
      this.rawSend(server, { jsonrpc: "2.0", method: "initialized", params: {} });
      const queued = server.queue.splice(0);
      for (const queuedMessage of queued) this.rawSend(server, queuedMessage);
      return;
    }

    // Server→client request: must answer or some servers (rust-analyzer) stall.
    if (message.method && message.id !== undefined) {
      let result: unknown = null;
      if (message.method === "workspace/configuration") {
        const params = message.params as { items?: unknown[] } | undefined;
        result = (params?.items ?? []).map(() => null);
      }
      this.rawSend(server, { jsonrpc: "2.0", id: message.id, result });
      return;
    }

    if (message.method === "textDocument/publishDiagnostics") {
      this.applyDiagnostics(message.params as PublishDiagnosticsParams);
    }
  }

  private applyDiagnostics(params: PublishDiagnosticsParams) {
    if (!params?.uri) return;
    const target = monaco.Uri.parse(params.uri).toString();
    const model = monaco.editor.getModels().find((candidate) => candidate.uri.toString() === target);
    if (!model) return;
    const markers = (params.diagnostics ?? []).map((diagnostic) => ({
      severity: LSP_SEVERITY[diagnostic.severity ?? 1] ?? monaco.MarkerSeverity.Error,
      message: diagnostic.source ? `${diagnostic.message} (${diagnostic.source})` : diagnostic.message,
      startLineNumber: diagnostic.range.start.line + 1,
      startColumn: diagnostic.range.start.character + 1,
      endLineNumber: diagnostic.range.end.line + 1,
      endColumn: diagnostic.range.end.character + 1,
    }));
    monaco.editor.setModelMarkers(model, "lsp", markers);
  }

  /** Open (or re-sync) a file in its language server. */
  async openFile(language: string, projectPath: string, uri: string, text: string) {
    if (!this.enabled) return;
    const serverLang = serverLangFor(language);
    if (!serverLang || !projectPath) return;
    const server = await this.ensureServer(serverLang, projectPath);
    if (!server) return;

    const existing = this.docs.get(uri);
    if (existing) {
      this.changeFile(uri, text);
      return;
    }
    this.docs.set(uri, { serverKey: server.key, version: 1 });
    this.notify(server, "textDocument/didOpen", {
      textDocument: { uri, languageId: language, version: 1, text },
    });
  }

  changeFile(uri: string, text: string) {
    const doc = this.docs.get(uri);
    if (!doc) return;
    const server = this.servers.get(doc.serverKey);
    if (!server || !server.available) return;
    doc.version += 1;
    this.notify(server, "textDocument/didChange", {
      textDocument: { uri, version: doc.version },
      contentChanges: [{ text }],
    });
  }

  closeFile(uri: string) {
    const doc = this.docs.get(uri);
    if (!doc) return;
    const server = this.servers.get(doc.serverKey);
    if (server?.available) {
      this.notify(server, "textDocument/didClose", { textDocument: { uri } });
    }
    this.docs.delete(uri);
    const model = monaco.editor.getModels().find((candidate) => candidate.uri.toString() === uri);
    if (model) monaco.editor.setModelMarkers(model, "lsp", []);
  }

  /** Stop every server (e.g. on project switch) and clear diagnostics. */
  reset() {
    for (const server of this.servers.values()) {
      if (server.id !== null) void api.lsp_stop(server.id);
    }
    this.servers.clear();
    this.docs.clear();
    for (const model of monaco.editor.getModels()) {
      monaco.editor.setModelMarkers(model, "lsp", []);
    }
  }
}

interface PublishDiagnosticsParams {
  uri: string;
  diagnostics?: {
    range: { start: { line: number; character: number }; end: { line: number; character: number } };
    severity?: number;
    message: string;
    source?: string;
  }[];
}

export const lspController = new LspController();
