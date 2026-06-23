import { describe, expect, it } from "vitest";
import {
  buildCommitMessagePrompt,
  aiCommitProfileKey,
  cleanCommitMessage,
  composeCommitMessage,
  latestAssistantCommitMessage,
  latestSystemMessage,
  resolveAiCommitProfileId,
  splitCommitMessage,
} from "./aiCommit";

describe("buildCommitMessagePrompt", () => {
  it("includes the diff and asks for a Conventional Commits message", () => {
    const prompt = buildCommitMessagePrompt("diff --git a/x b/x", {
      projectName: "clia-app",
      stagedFiles: [{ path: "src/App.tsx", status: "M", additions: 10, deletions: 2 }],
    });
    expect(prompt).toContain("Conventional Commits");
    expect(prompt).toContain("inspirado no comando /dw-commit");
    expect(prompt).toContain("Analise SOMENTE as mudanças STAGED");
    expect(prompt).toContain("Projeto: clia-app");
    expect(prompt).toContain("M  src/App.tsx +10 -2");
    expect(prompt).toContain("diff --git a/x b/x");
  });

  it("forbids mutating git actions", () => {
    const prompt = buildCommitMessagePrompt("diff --git a/x b/x");
    expect(prompt).toContain("Não execute git commit");
    expect(prompt).toContain("git add");
    expect(prompt).toContain("git restore");
  });

  it("pushes the agent away from vague commit messages", () => {
    const prompt = buildCommitMessagePrompt(".dw/flows/index.json | 20 ++");
    expect(prompt).toContain("type válido");
    expect(prompt).toContain("scope curto e específico");
    expect(prompt).toContain("Não use mensagens vagas");
    expect(prompt).toContain("regras e fluxos");
    expect(prompt).toContain("descrição curta");
  });

  it("truncates very large diffs", () => {
    const prompt = buildCommitMessagePrompt("x".repeat(20000));
    expect(prompt).toContain("diff truncado");
    expect(prompt.length).toBeLessThan(14500);
  });
});

describe("cleanCommitMessage", () => {
  it("returns a bare message unchanged", () => {
    expect(cleanCommitMessage("feat(git): add AI commit")).toBe("feat(git): add AI commit");
  });

  it("strips a fenced block", () => {
    expect(cleanCommitMessage("```\nfeat: x\n\nbody line\n```")).toBe("feat: x\n\nbody line");
  });

  it("strips wrapping quotes", () => {
    expect(cleanCommitMessage('"fix: y"')).toBe("fix: y");
  });

  it("strips subject and description labels when the agent disobeys the format", () => {
    expect(
      cleanCommitMessage("Subject: feat(git): add menu\nDescription: Add staged actions"),
    ).toBe("feat(git): add menu\n\nAdd staged actions");
  });
});

describe("latestAssistantCommitMessage", () => {
  it("returns the last assistant message cleaned for a recovered AI commit session", () => {
    expect(
      latestAssistantCommitMessage([
        { role: "user", content: "prompt" },
        { role: "assistant", content: "```\nfeat(gui): first\n```" },
        { role: "event", content: "" },
        { role: "assistant", content: '"fix(git): use staged diff"' },
      ]),
    ).toBe("fix(git): use staged diff");
  });

  it("returns null when the session has no assistant output", () => {
    expect(
      latestAssistantCommitMessage([
        { role: "user", content: "prompt" },
        { role: "system", content: "Codex failed" },
      ]),
    ).toBeNull();
  });
});

describe("latestSystemMessage", () => {
  it("returns the last system error for a failed AI commit session", () => {
    expect(
      latestSystemMessage([
        { role: "system", content: "old" },
        { role: "system", content: "Missing optional dependency" },
      ]),
    ).toBe("Missing optional dependency");
  });
});

describe("commit message field helpers", () => {
  it("splits a subject-only message", () => {
    expect(splitCommitMessage("fix(git): use staged diff")).toEqual({
      subject: "fix(git): use staged diff",
      description: "",
    });
  });

  it("splits subject and body", () => {
    expect(splitCommitMessage("feat(git): add AI commit\n\nGenerate from staged diff.")).toEqual({
      subject: "feat(git): add AI commit",
      description: "Generate from staged diff.",
    });
  });

  it("composes subject and description with a blank line", () => {
    expect(composeCommitMessage("feat(git): add AI commit", "Generate from staged diff.")).toBe(
      "feat(git): add AI commit\n\nGenerate from staged diff.",
    );
  });

  it("composes subject-only messages without extra spacing", () => {
    expect(composeCommitMessage("fix(git): use staged diff", "")).toBe("fix(git): use staged diff");
  });
});

describe("AI Commit profile selection", () => {
  it("uses a stable per-project app-state key", () => {
    expect(aiCommitProfileKey(42)).toBe("ai_commit_profile:42");
  });

  it("prefers the saved profile when it still exists", () => {
    expect(resolveAiCommitProfileId([{ id: 1 }, { id: 2 }], 2, 1)).toBe(2);
  });

  it("falls back to the active profile, then the first profile", () => {
    expect(resolveAiCommitProfileId([{ id: 1 }, { id: 2 }], 99, 2)).toBe(2);
    expect(resolveAiCommitProfileId([{ id: 1 }, { id: 2 }], null, 99)).toBe(1);
    expect(resolveAiCommitProfileId([], null, null)).toBeNull();
  });
});
