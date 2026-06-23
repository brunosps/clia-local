import { describe, expect, it } from "vitest";
import {
  parseProjectBlueprintAgentResponse,
  projectBlueprintQuestionBatch,
  PROJECT_BLUEPRINT_BATCH_SIZE,
  PROJECT_BLUEPRINT_QUESTION_BANK,
} from "./projectBlueprint";

describe("project blueprint interview", () => {
  it("keeps the configured question bank and batch size bounded", () => {
    expect(PROJECT_BLUEPRINT_QUESTION_BANK).toHaveLength(77);
    expect(PROJECT_BLUEPRINT_BATCH_SIZE).toBe(3);
    expect(projectBlueprintQuestionBatch(0)).toHaveLength(3);
    expect(projectBlueprintQuestionBatch(76)).toHaveLength(1);
    expect(projectBlueprintQuestionBatch(77)).toHaveLength(0);
  });

  it("parses strict question batch JSON", () => {
    const parsed = parseProjectBlueprintAgentResponse(
      'prefix {"state":"question_batch","running_summary":"sum","detected_subprojects":["api"],"questions":[{"id":"business-01","area":"Negócio","question":"Problema?"}]} suffix',
    );
    expect(parsed).toEqual({
      state: "question_batch",
      running_summary: "sum",
      detected_subprojects: ["api"],
      questions: [{ id: "business-01", area: "Negócio", question: "Problema?" }],
    });
  });

  it("parses strict final plan JSON", () => {
    const parsed = parseProjectBlueprintAgentResponse(
      JSON.stringify({
        state: "final_plan",
        running_summary: "ready",
        detected_subprojects: ["frontend", "backend"],
        prd: "# PRD",
        techspec: "# TechSpec",
        tasks: [{ title: "Task one", body: "Build it" }],
        definition_of_done: "# DoD",
      }),
    );
    expect(parsed).toEqual({
      state: "final_plan",
      running_summary: "ready",
      detected_subprojects: ["frontend", "backend"],
      prd: "# PRD",
      techspec: "# TechSpec",
      tasks: [{ title: "Task one", body: "Build it", dependencies: undefined }],
      definition_of_done: "# DoD",
    });
  });
});
