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
  cachedTokens: number;
  premiumRequests: number;
  nanoAiu: number;
  costUsd: number;
};

type UsageMetric = { run_id?: string; phase: string; details_json: string };

function parseDetails(json: string): Record<string, unknown> | null {
  try {
    const parsed: unknown = JSON.parse(json);
    return typeof parsed === "object" && parsed !== null
      ? (parsed as Record<string, unknown>)
      : null;
  } catch {
    return null;
  }
}

function numeric(record: Record<string, unknown>, key: string): number {
  const value = record[key];
  return typeof value === "number" && Number.isFinite(value) ? value : 0;
}

/**
 * Running usage for a whole conversation, not just the last turn.
 *
 * The providers do not agree on what `input_tokens` means, and getting this wrong
 * is not a rounding error — it is the difference between "29k" and "2". Verified
 * against each CLI's real output:
 *
 *   - Claude: `input_tokens` counts ONLY the uncached part. A real turn came back
 *     as input 2 / cache_creation 14216 / cache_read 15251, so the cache fields
 *     have to be added to get the input actually processed.
 *   - Codex: `input_tokens` is the total (15473) and `cached_input_tokens` (13056)
 *     is the slice of it that was cached — adding it would double count.
 *   - Copilot: no input tokens at all; it reports output tokens per message plus
 *     premium requests and AI Units.
 *
 * AI Units arrive as a per-run cumulative checkpoint, so they are maxed within a
 * run and summed across runs; everything else is a per-turn delta and is summed.
 */
export function sumSessionTokenUsage(metrics: UsageMetric[]): SessionTokenUsage {
  const total: SessionTokenUsage = {
    inputTokens: 0,
    outputTokens: 0,
    cachedTokens: 0,
    premiumRequests: 0,
    nanoAiu: 0,
    costUsd: 0,
  };
  const aiuByRun = new Map<string, number>();

  for (const metric of metrics) {
    const details = parseDetails(metric.details_json);
    if (!details) continue;

    if (metric.phase === "usage_checkpoint") {
      const run = metric.run_id ?? "";
      aiuByRun.set(run, Math.max(aiuByRun.get(run) ?? 0, numeric(details, "nano_aiu")));
      continue;
    }
    if (metric.phase !== "result") continue;

    const cacheCreate = numeric(details, "cache_creation_input_tokens");
    const cacheRead = numeric(details, "cache_read_input_tokens");
    total.inputTokens += numeric(details, "input_tokens") + cacheCreate + cacheRead;
    total.outputTokens += numeric(details, "output_tokens");
    // `cached_input_tokens` (Codex) is already inside input_tokens; the Claude
    // cache_read is not. Both are reported here only as "how much was cached".
    total.cachedTokens += cacheRead + numeric(details, "cached_input_tokens");
    total.premiumRequests += numeric(details, "premium_requests");
    total.costUsd += numeric(details, "total_cost_usd");
  }

  for (const value of aiuByRun.values()) total.nanoAiu += value;
  return total;
}

/** Compact label for the session header; empty when the provider reported nothing. */
export function formatSessionTokenUsage(usage: SessionTokenUsage): string {
  const compact = (value: number) =>
    value >= 1000 ? `${(value / 1000).toFixed(value >= 10000 ? 0 : 1)}k` : String(value);
  const parts: string[] = [];
  if (usage.inputTokens > 0 || usage.outputTokens > 0) {
    parts.push(
      usage.inputTokens > 0
        ? `${compact(usage.inputTokens)}↑ ${compact(usage.outputTokens)}↓ tokens`
        : `${compact(usage.outputTokens)} tokens`,
    );
  }
  if (usage.cachedTokens > 0) parts.push(`${compact(usage.cachedTokens)} cache`);
  if (usage.nanoAiu > 0) parts.push(`${(usage.nanoAiu / 1e9).toFixed(2)} AIU`);
  if (usage.premiumRequests > 0) parts.push(`${usage.premiumRequests} premium`);
  if (usage.costUsd > 0) parts.push(`$${usage.costUsd.toFixed(4)}`);
  return parts.join(" · ");
}
