import type { AgentSession } from "./types";

export function isAgentRunning(session: Pick<AgentSession, "status"> | null) {
  return session?.status === "running";
}

export function hasRunningAgentSession(sessions: Array<Pick<AgentSession, "status">>) {
  return sessions.some((session) => isAgentRunning(session));
}

export function upsertAgentSession(sessions: AgentSession[], next: AgentSession) {
  if (next.scope && next.scope !== "chat") {
    return sessions.filter((session) => session.id !== next.id);
  }
  const known = sessions.some((session) => session.id === next.id);
  const updated = known
    ? sessions.map((session) => (session.id === next.id ? next : session))
    : [next, ...sessions];
  return updated.sort((left, right) => right.updated_at.localeCompare(left.updated_at));
}

export function agentSessionsForProfile(sessions: AgentSession[], profileId: number | null) {
  if (!profileId) return [];
  return sessions.filter(
    (session) => session.profile_id === profileId && (!session.scope || session.scope === "chat"),
  );
}

export function agentSessionBelongsToProfile(
  session: Pick<AgentSession, "profile_id"> | null,
  profileId: number | null,
) {
  return Boolean(session && profileId && session.profile_id === profileId);
}

export function resolveActiveAgentSession(
  sessions: AgentSession[],
  profileId: number | null,
  sessionId: number | null,
) {
  const profileSessions = agentSessionsForProfile(sessions, profileId);
  return profileSessions.find((session) => session.id === sessionId) ?? profileSessions[0] ?? null;
}

export function shouldAppendAgentMessage(activeSessionId: number | null, eventSessionId: number) {
  return Boolean(activeSessionId && activeSessionId === eventSessionId);
}

export function agentStatusLabel(status: string) {
  switch (status) {
    case "idle":
      return "Idle";
    case "running":
      return "Working";
    case "done":
      return "Done";
    case "failed":
      return "Failed";
    case "stopped":
      return "Stopped";
    default:
      return status;
  }
}
