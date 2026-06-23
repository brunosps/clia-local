import type { TerminalOutputEvent, TerminalSession, TerminalStatusEvent } from "./types";

export const TERMINAL_SCROLLBACK_LINES = 10_000;

export type TerminalBuffers = Record<string, string>;

export function isTerminalRunning(session: TerminalSession | null) {
  return session?.status === "running";
}

export function terminalStatusLabel(status: TerminalSession["status"]) {
  switch (status) {
    case "idle":
      return "Idle";
    case "running":
      return "Running";
    case "exited":
      return "Exited";
    case "failed":
      return "Failed";
    case "stopped":
      return "Stopped";
  }
}

export function appendTerminalOutput(
  buffers: TerminalBuffers,
  event: TerminalOutputEvent,
  maxLines = TERMINAL_SCROLLBACK_LINES,
) {
  const current = buffers[event.session_id] ?? "";
  const next = `${current}${event.data}`;
  return {
    ...buffers,
    [event.session_id]: trimScrollback(next, maxLines),
  };
}

export function applyTerminalStatus(
  sessions: TerminalSession[],
  event: TerminalStatusEvent,
): TerminalSession[] {
  return sessions.map((session) =>
    session.id === event.session_id
      ? {
          ...session,
          status: event.status,
          exit_code: event.exit_code ?? null,
          updated_at: new Date().toISOString(),
        }
      : session,
  );
}

export function upsertTerminalSession(sessions: TerminalSession[], next: TerminalSession) {
  const existing = sessions.some((session) => session.id === next.id);
  if (!existing) return [...sessions, next];
  return sessions.map((session) => (session.id === next.id ? next : session));
}

export function normalizeComparablePath(path: string) {
  const normalized = path.trim().replace(/\\/g, "/");
  return normalized.length > 1 ? normalized.replace(/\/+$/g, "") : normalized;
}

export function findTerminalForPath(
  sessions: TerminalSession[],
  path: string,
  runningOnly = false,
) {
  const normalizedPath = normalizeComparablePath(path);
  if (!normalizedPath) return null;
  return (
    sessions.find(
      (session) =>
        normalizeComparablePath(session.cwd) === normalizedPath &&
        (!runningOnly || isTerminalRunning(session)),
    ) ?? null
  );
}

function trimScrollback(value: string, maxLines: number) {
  const lines = value.split("\n");
  if (lines.length <= maxLines) return value;
  return lines.slice(lines.length - maxLines).join("\n");
}
