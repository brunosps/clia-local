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

export type SessionTokenUsage = {
  inputTokens: number;
  outputTokens: number;
  premiumRequests: number;
};

/**
 * Running token total for a whole conversation, not just the last turn.
 *
 * Sums every `result` event in the session. Providers report different subsets —
 * Copilot only gives output tokens plus a premium-request count, Codex/Claude give
 * input and output — so a zero here means "not reported", never "nothing spent".
 */
export function sumSessionTokenUsage(
  metrics: Array<{ phase: string; details_json: string }>,
): SessionTokenUsage {
  const total: SessionTokenUsage = { inputTokens: 0, outputTokens: 0, premiumRequests: 0 };
  for (const metric of metrics) {
    if (metric.phase !== "result") continue;
    let details: unknown;
    try {
      details = JSON.parse(metric.details_json);
    } catch {
      continue;
    }
    if (typeof details !== "object" || details === null) continue;
    const record = details as Record<string, unknown>;
    const read = (key: string) => {
      const value = record[key];
      return typeof value === "number" && Number.isFinite(value) ? value : 0;
    };
    total.inputTokens += read("input_tokens");
    total.outputTokens += read("output_tokens");
    total.premiumRequests += read("premium_requests");
  }
  return total;
}

/** Compact label for the session header; empty when the provider reported nothing. */
export function formatSessionTokenUsage(usage: SessionTokenUsage): string {
  const compact = (value: number) =>
    value >= 1000 ? `${(value / 1000).toFixed(value >= 10000 ? 0 : 1)}k` : String(value);
  const parts: string[] = [];
  const tokens = usage.inputTokens + usage.outputTokens;
  if (tokens > 0) {
    parts.push(
      usage.inputTokens > 0
        ? `${compact(usage.inputTokens)}↑ ${compact(usage.outputTokens)}↓ tokens`
        : `${compact(usage.outputTokens)} tokens`,
    );
  }
  if (usage.premiumRequests > 0) {
    parts.push(`${usage.premiumRequests} premium`);
  }
  return parts.join(" · ");
}
