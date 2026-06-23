import { describe, expect, it } from "vitest";
import { analyzeInterviewPrompt, parseAnalyzeInterviewResponse } from "./analyzeInterview";

describe("parseAnalyzeInterviewResponse", () => {
  it("parses a question with options", () => {
    const parsed = parseAnalyzeInterviewResponse(
      'prefix ```json\n{"state":"question","question":"Foco?","options":["UI","API",""]}\n``` suffix',
    );
    expect(parsed).toEqual({ state: "question", question: "Foco?", options: ["UI", "API"] });
  });

  it("parses an open question (no options)", () => {
    const parsed = parseAnalyzeInterviewResponse('{"state":"question","question":"Convenções?"}');
    expect(parsed).toEqual({ state: "question", question: "Convenções?", options: undefined });
  });

  it("parses the done state", () => {
    expect(parseAnalyzeInterviewResponse('{"state":"done"}')).toEqual({ state: "done" });
  });

  it("treats the legacy final state as done", () => {
    expect(parseAnalyzeInterviewResponse('{"state":"final"}')).toEqual({ state: "done" });
  });

  it("parses a working/progress update", () => {
    expect(parseAnalyzeInterviewResponse('{"state":"working","message":"escrevendo index.md"}')).toEqual({
      state: "working",
      message: "escrevendo index.md",
    });
  });

  it("returns null for non-JSON or unknown state", () => {
    expect(parseAnalyzeInterviewResponse("sem json aqui")).toBeNull();
    expect(parseAnalyzeInterviewResponse('{"state":"question"}')).toBeNull(); // missing question
    expect(parseAnalyzeInterviewResponse('{"state":"other"}')).toBeNull();
  });
});

describe("analyzeInterviewPrompt", () => {
  it("names the project and command and asks for JSON one question at a time", () => {
    const prompt = analyzeInterviewPrompt({ projectName: "winbox-gui", analyzeCommand: "/dw-analyze-project" });
    expect(prompt).toContain("winbox-gui");
    expect(prompt).toContain("/dw-analyze-project");
    expect(prompt).toContain('"state":"question"');
    expect(prompt).toContain('"state":"done"');
  });
});
