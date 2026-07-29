import { describe, expect, it } from "vitest";

import { formatSessionTokenUsage, sumSessionTokenUsage } from "./agents";

const metric = (phase: string, details: Record<string, unknown>, run_id = "run-1") => ({
  run_id,
  phase,
  details_json: JSON.stringify(details),
});

describe("session token usage", () => {
  // Payload copied from a real `claude -p --output-format stream-json` result:
  // input_tokens counts only what was NOT cached, so the cache fields are part of
  // the input actually processed and have to be added.
  it("adds Claude's cache counters, which sit outside input_tokens", () => {
    const usage = sumSessionTokenUsage([
      metric("result", {
        input_tokens: 2,
        cache_creation_input_tokens: 14216,
        cache_read_input_tokens: 15251,
        output_tokens: 15,
        total_cost_usd: 0.300937,
      }),
    ]);
    expect(usage.inputTokens).toBe(29469);
    expect(usage.outputTokens).toBe(15);
    expect(usage.cachedTokens).toBe(15251);
    expect(usage.costUsd).toBeCloseTo(0.300937, 6);
  });

  // From a real `codex exec --json` turn.completed: cached_input_tokens is a slice
  // of input_tokens (13056 of 15473), so adding it would double count.
  it("does not double count Codex's cached_input_tokens", () => {
    const usage = sumSessionTokenUsage([
      metric("result", {
        input_tokens: 15473,
        cached_input_tokens: 13056,
        cache_write_input_tokens: 0,
        output_tokens: 5,
        reasoning_output_tokens: 0,
      }),
    ]);
    expect(usage.inputTokens).toBe(15473);
    expect(usage.outputTokens).toBe(5);
    expect(usage.cachedTokens).toBe(13056);
  });

  it("accumulates across every turn of the conversation", () => {
    const usage = sumSessionTokenUsage([
      metric("result", { input_tokens: 1200, output_tokens: 300 }),
      metric("provider_init", { tools: 12 }),
      metric("result", { input_tokens: 800, output_tokens: 150 }),
    ]);
    expect(usage.inputTokens).toBe(2000);
    expect(usage.outputTokens).toBe(450);
  });

  it("handles the Copilot shape: output tokens, premium requests, AI Units", () => {
    const usage = sumSessionTokenUsage([
      metric("result", { output_tokens: 4 }),
      metric("result", { output_tokens: 120 }),
      metric("result", { event_type: "result", premium_requests: 1 }),
      metric("usage_checkpoint", { nano_aiu: 6050150000 }),
    ]);
    expect(usage.inputTokens).toBe(0);
    expect(usage.outputTokens).toBe(124);
    expect(usage.premiumRequests).toBe(1);
    expect(usage.nanoAiu).toBe(6050150000);
  });

  // AI Units come as a cumulative checkpoint for the run, so several checkpoints in
  // one run must not stack; separate runs must.
  it("maxes AI Units within a run and sums across runs", () => {
    const usage = sumSessionTokenUsage([
      metric("usage_checkpoint", { nano_aiu: 1_000_000_000 }, "run-1"),
      metric("usage_checkpoint", { nano_aiu: 3_000_000_000 }, "run-1"),
      metric("usage_checkpoint", { nano_aiu: 2_000_000_000 }, "run-2"),
    ]);
    expect(usage.nanoAiu).toBe(5_000_000_000);
  });

  it("ignores malformed or non-numeric details instead of throwing", () => {
    const usage = sumSessionTokenUsage([
      { run_id: "r", phase: "result", details_json: "not json" },
      { run_id: "r", phase: "result", details_json: "null" },
      metric("result", { input_tokens: "600", output_tokens: null }),
      metric("result", { input_tokens: 10 }),
    ]);
    expect(usage.inputTokens).toBe(10);
    expect(usage.outputTokens).toBe(0);
  });

  it("formats compactly and stays empty when nothing was reported", () => {
    const empty = {
      inputTokens: 0,
      outputTokens: 0,
      cachedTokens: 0,
      premiumRequests: 0,
      nanoAiu: 0,
      costUsd: 0,
    };
    expect(formatSessionTokenUsage(empty)).toBe("");
    expect(formatSessionTokenUsage({ ...empty, outputTokens: 124, premiumRequests: 1 })).toBe(
      "124 tokens · 1 premium",
    );
    expect(formatSessionTokenUsage({ ...empty, inputTokens: 29469, outputTokens: 15 })).toBe(
      "29k↑ 15↓ tokens",
    );
    expect(formatSessionTokenUsage({ ...empty, nanoAiu: 6050150000 })).toBe("6.05 AIU");
    expect(formatSessionTokenUsage({ ...empty, inputTokens: 2000, costUsd: 0.3009 })).toBe(
      "2.0k↑ 0↓ tokens · $0.3009",
    );
  });
});
