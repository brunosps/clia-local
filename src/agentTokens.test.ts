import { describe, expect, it } from "vitest";

import { formatSessionTokenUsage, sumSessionTokenUsage } from "./agents";

const metric = (phase: string, details: Record<string, unknown>) => ({
  phase,
  details_json: JSON.stringify(details),
});

describe("session token usage", () => {
  it("accumulates across every result event of the conversation", () => {
    const usage = sumSessionTokenUsage([
      metric("result", { input_tokens: 1200, output_tokens: 300 }),
      metric("provider_init", { tools: 12 }),
      metric("result", { input_tokens: 800, output_tokens: 150 }),
    ]);
    expect(usage).toEqual({ inputTokens: 2000, outputTokens: 450, premiumRequests: 0 });
  });

  it("handles the Copilot shape, which reports output tokens and premium requests only", () => {
    const usage = sumSessionTokenUsage([
      metric("result", { output_tokens: 4 }),
      metric("result", { output_tokens: 120 }),
      metric("result", { event_type: "result", premium_requests: 1 }),
    ]);
    expect(usage).toEqual({ inputTokens: 0, outputTokens: 124, premiumRequests: 1 });
  });

  it("ignores malformed or non-numeric details instead of throwing", () => {
    const usage = sumSessionTokenUsage([
      { phase: "result", details_json: "not json" },
      { phase: "result", details_json: "null" },
      metric("result", { input_tokens: "600", output_tokens: null }),
      metric("result", { input_tokens: 10 }),
    ]);
    expect(usage).toEqual({ inputTokens: 10, outputTokens: 0, premiumRequests: 0 });
  });

  it("formats compactly and stays empty when nothing was reported", () => {
    expect(formatSessionTokenUsage({ inputTokens: 0, outputTokens: 0, premiumRequests: 0 })).toBe(
      "",
    );
    expect(formatSessionTokenUsage({ inputTokens: 0, outputTokens: 124, premiumRequests: 1 })).toBe(
      "124 tokens · 1 premium",
    );
    expect(
      formatSessionTokenUsage({ inputTokens: 2000, outputTokens: 450, premiumRequests: 0 }),
    ).toBe("2.0k↑ 450↓ tokens");
    expect(
      formatSessionTokenUsage({ inputTokens: 45000, outputTokens: 12000, premiumRequests: 0 }),
    ).toBe("45k↑ 12k↓ tokens");
  });
});
