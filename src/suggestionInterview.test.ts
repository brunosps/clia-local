import { describe, expect, it } from "vitest";
import { parseSuggestionInterviewResponse, suggestionInterviewPrompt } from "./suggestionInterview";

describe("parseSuggestionInterviewResponse", () => {
  it("parses a question with options", () => {
    const parsed = parseSuggestionInterviewResponse(
      'prefix ```json\n{"state":"question","question":"Foco?","options":["UI","API",""]}\n``` suffix',
    );
    expect(parsed).toEqual({ state: "question", question: "Foco?", options: ["UI", "API"] });
  });

  it("parses an open question (no options)", () => {
    const parsed = parseSuggestionInterviewResponse('{"state":"question","question":"Prioridade?"}');
    expect(parsed).toEqual({ state: "question", question: "Prioridade?", options: undefined });
  });

  it("parses a working/progress update", () => {
    expect(parseSuggestionInterviewResponse('{"state":"working","message":"pensando"}')).toEqual({
      state: "working",
      message: "pensando",
    });
  });

  it("parses done with a list of suggestions, dropping invalid items", () => {
    const parsed = parseSuggestionInterviewResponse(
      '{"state":"done","suggestions":[' +
        '{"title":"Atalho de busca","body":"Cmd+K global.","kind":"feature"},' +
        '{"title":"Extrair hook","body":"","kind":"refactor"},' +
        '{"body":"sem título"},' +
        '{"title":"Sem kind","body":"ok","kind":"weird"}' +
        "]}",
    );
    expect(parsed).toEqual({
      state: "done",
      suggestions: [
        { title: "Atalho de busca", body: "Cmd+K global.", kind: "feature" },
        { title: "Extrair hook", body: "", kind: "refactor" },
        { title: "Sem kind", body: "ok", kind: undefined },
      ],
    });
  });

  it("parses done without a suggestions array as an empty list", () => {
    expect(parseSuggestionInterviewResponse('{"state":"done"}')).toEqual({ state: "done", suggestions: [] });
  });

  it("returns null for non-JSON or unknown state", () => {
    expect(parseSuggestionInterviewResponse("sem json aqui")).toBeNull();
    expect(parseSuggestionInterviewResponse('{"state":"question"}')).toBeNull(); // missing question
    expect(parseSuggestionInterviewResponse('{"state":"other"}')).toBeNull();
  });
});

describe("suggestionInterviewPrompt", () => {
  it("names the project and command and asks for JSON, ending with a done list", () => {
    const prompt = suggestionInterviewPrompt({ projectName: "winbox-gui", suggestCommand: "/dw-opportunities" });
    expect(prompt).toContain("winbox-gui");
    expect(prompt).toContain("/dw-opportunities");
    expect(prompt).toContain('"state":"question"');
    expect(prompt).toContain('"state":"done"');
    expect(prompt).toContain("suggestions");
  });
});
