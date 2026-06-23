import { describe, expect, it } from "vitest";
import { flowInterviewPrompt, parseFlowInterviewResponse } from "./flowInterview";

describe("parseFlowInterviewResponse", () => {
  it("parses a question with H1-H4 options", () => {
    const parsed = parseFlowInterviewResponse(
      JSON.stringify({
        state: "question",
        question_number: 1,
        question: "Quais fases?",
        options: { H1: "a", H2: "b", H3: "c", H4: "d" },
      }),
    );
    expect(parsed?.state).toBe("question");
    if (parsed?.state === "question") expect(parsed.options.H1).toBe("a");
  });

  it("parses a final with an inline flow object (stringified)", () => {
    const parsed = parseFlowInterviewResponse(
      JSON.stringify({
        state: "final",
        id: "tool",
        label: "Tool",
        flow: { version: 1, phases: [{ id: "p", label: "P", status: "p", action: { type: "none" } }] },
      }),
    );
    expect(parsed?.state).toBe("final");
    if (parsed?.state === "final") {
      expect(parsed.id).toBe("tool");
      expect(parsed.flow).toContain('"phases"');
    }
  });

  it("parses a final with a fenced flow and ignores prose around it", () => {
    const parsed = parseFlowInterviewResponse(
      'Aqui está:\n```json\n{"state":"final","flow":{"version":1,"phases":[]}}\n```\nfim',
    );
    expect(parsed?.state).toBe("final");
  });

  it("returns null for non-JSON or unknown state", () => {
    expect(parseFlowInterviewResponse("sem json aqui")).toBeNull();
    expect(parseFlowInterviewResponse(JSON.stringify({ state: "final" }))).toBeNull();
  });
});

describe("flowInterviewPrompt", () => {
  it("embeds page content when provided and asks for WebFetch otherwise", () => {
    const withContent = flowInterviewPrompt({ url: "https://x", pageContent: "DOCS", turns: [] });
    expect(withContent).toContain("DOCS");
    const without = flowInterviewPrompt({ url: "https://x", turns: [] });
    expect(without).toContain("WebFetch");
  });

  it("renders prior turns", () => {
    const prompt = flowInterviewPrompt({
      url: "https://x",
      turns: [{ question: "Q1", selected: "H2", answer: "resp", note: "nota" }],
    });
    expect(prompt).toContain("Q1");
    expect(prompt).toContain("resp");
  });
});
