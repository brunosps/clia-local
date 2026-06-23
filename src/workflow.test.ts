import { describe, expect, it } from "vitest";
import {
  agentSessionBelongsToProfile,
  agentSessionsForProfile,
  agentStatusLabel,
  hasRunningAgentSession,
  isAgentRunning,
  resolveActiveAgentSession,
  shouldAppendAgentMessage,
  upsertAgentSession,
} from "./agents";
import { artifactCounts, artifactLanguage, formatArtifactSize } from "./artifacts";
import { evidenceCounts, evidenceStatusLabel, parseEvidenceLinks } from "./evidence";
import { parseInterviewAgentResponse } from "./interview";
import {
  canStageHunks,
  countPatchAreas,
  fileActionLabel,
  formatPatchStats,
  groupChangedFiles,
} from "./patches";
import { countSourceFiles, formatSourceSize, sourceLanguage } from "./source";
import {
  appendTerminalOutput,
  applyTerminalStatus,
  findTerminalForPath,
  isTerminalRunning,
  normalizeComparablePath,
  terminalStatusLabel,
  TERMINAL_SCROLLBACK_LINES,
  upsertTerminalSession,
} from "./terminal";
import {
  composeDwPlanCommand,
  isArchivedRequirement,
  quoteCommandArg,
  requirementPipelineCommands,
  requirementPrdSlug,
  stageCounts,
  workflowStages,
} from "./workflow";
import {
  createDefaultWorkspaceRoot,
  parseStateId,
  pickActiveProject,
  pickActiveWorkspace,
  projectDisplayName,
} from "./workspace";

describe("workflow model", () => {
  it("keeps the dev-workflow pipeline in order", () => {
    expect(workflowStages.map((stage) => stage.command)).toEqual([
      "/dw-brainstorm",
      "/dw-plan",
      "/dw-run",
      "/dw-qa",
      "/dw-review",
      "/dw-commit",
      "/dw-generate-pr",
    ]);
  });

  it("summarizes stage states", () => {
    expect(stageCounts()).toEqual({ ready: 2, active: 0, complete: 0, pending: 1, blocked: 4 });
  });

  it("composes dw-plan commands with quoted input and skill context", () => {
    expect(composeDwPlanCommand("default", "runner ui")).toBe('/dw-plan "runner ui"');
    expect(composeDwPlanCommand("prd", 'say "hello"', [{ name: "dw-ui-discipline" }])).toBe(
      '/dw-plan prd "say \\"hello\\"" $dw-ui-discipline',
    );
  });

  it("quotes command arguments without losing backslashes", () => {
    expect(quoteCommandArg(String.raw`C:\repo\feature`)).toBe('"C:\\\\repo\\\\feature"');
  });

  it("builds requirement-scoped workflow commands", () => {
    const card = {
      id: 1,
      workspace_id: 1,
      project_id: 1,
      title: "Local PR package",
      slug: "local-pr-package",
      body: "",
      status: "draft",
      prd_slug: null,
      created_at: "2026-05-20T00:00:00Z",
      updated_at: "2026-05-20T00:00:00Z",
    };

    expect(requirementPrdSlug(card)).toBe("prd-local-pr-package");
    expect(requirementPipelineCommands(card).map((item) => item.command)).toEqual([
      '/dw-brainstorm "Local PR package"',
      '/dw-plan "Local PR package"',
      '/dw-goal "prd-local-pr-package"',
      '/dw-run "prd-local-pr-package"',
      '/dw-review "prd-local-pr-package"',
      '/dw-qa "prd-local-pr-package"',
      "/dw-commit",
      "/dw-generate-pr",
    ]);
    expect(isArchivedRequirement({ status: "archived" })).toBe(true);
    // Per-stage fields, command composition, and stage lookup now live in the
    // workbench schema — see workbenchSchema.test.ts for that coverage.
  });
});

describe("agent session model", () => {
  const sessions = [
    {
      id: 1,
      profile_id: 10,
      workspace_id: 1,
      project_id: 1,
      requirement_card_id: null,
      scope: "chat",
      project_path: "/repo",
      provider: "codex",
      model: "gpt-5.4-mini",
      reasoning_effort: "medium",
      sandbox: "read-only",
      context_mode: "auto_lean",
      provider_session_id: null,
      codex_session_id: null,
      status: "done",
      title: "Codex done",
      created_at: "2026-05-23T10:00:00Z",
      updated_at: "2026-05-23T10:00:00Z",
    },
    {
      id: 2,
      profile_id: 20,
      workspace_id: 1,
      project_id: 1,
      requirement_card_id: null,
      scope: "chat",
      project_path: "/repo",
      provider: "claude",
      model: "claude-opus-4-6",
      reasoning_effort: "high",
      sandbox: "danger-full-access",
      context_mode: "auto_lean",
      provider_session_id: null,
      codex_session_id: null,
      status: "running",
      title: "Claude running",
      created_at: "2026-05-23T11:00:00Z",
      updated_at: "2026-05-23T11:00:00Z",
    },
  ];

  it("scopes active sessions and streamed messages to the selected agent", () => {
    expect(agentSessionsForProfile(sessions, 10).map((session) => session.id)).toEqual([1]);
    expect(resolveActiveAgentSession(sessions, 10, 2)?.id).toBe(1);
    expect(resolveActiveAgentSession(sessions, 20, 2)?.id).toBe(2);
    expect(resolveActiveAgentSession(sessions, null, 1)).toBeNull();
    expect(agentSessionBelongsToProfile(sessions[0], 10)).toBe(true);
    expect(agentSessionBelongsToProfile(sessions[0], 20)).toBe(false);
    expect(shouldAppendAgentMessage(1, 1)).toBe(true);
    expect(shouldAppendAgentMessage(1, 2)).toBe(false);
  });

  it("keeps card interview sessions out of the normal agent chat list", () => {
    const hidden = {
      ...sessions[0],
      id: 3,
      requirement_card_id: 99,
      scope: "card_interview",
      updated_at: "2026-05-23T12:00:00Z",
    };

    expect(agentSessionsForProfile([...sessions, hidden], 10).map((session) => session.id)).toEqual(
      [1],
    );
    expect(upsertAgentSession(sessions, hidden).map((session) => session.id)).toEqual([1, 2]);
  });
});

describe("backlog interview parser", () => {
  it("accepts a structured question with H1-H4 options", () => {
    expect(
      parseInterviewAgentResponse(`{
        "state": "question",
        "question_number": 1,
        "question": "Qual caminho faz sentido?",
        "options": {
          "H1": "Conservador com melhoria",
          "H2": "Mais ousado",
          "H3": "Disruptivo",
          "H4": "Nao fazer agora"
        },
        "running_summary": "Resumo"
      }`),
    ).toMatchObject({ state: "question", question_number: 1 });
  });

  it("rejects a question without all four options", () => {
    expect(
      parseInterviewAgentResponse(`{
        "state": "question",
        "question_number": 1,
        "question": "Qual caminho?",
        "options": { "H1": "A", "H2": "B", "H3": "C" }
      }`),
    ).toBeNull();
  });

  it("accepts a final draft with checklist", () => {
    expect(
      parseInterviewAgentResponse(`\`\`\`json
      {
        "state": "final",
        "description": "Descricao",
        "context": "Entendimento",
        "expected_result": "Resultado",
        "checklist": ["Validar fluxo"],
        "summary": "Resumo"
      }
      \`\`\``),
    ).toMatchObject({ state: "final", checklist: ["Validar fluxo"] });
  });
});

describe("artifact model", () => {
  it("summarizes artifacts by dev-workflow category", () => {
    expect(
      artifactCounts([
        { relative_path: "STATE.md", category: "state", name: "STATE.md", bytes: 10 },
        { relative_path: "spec/projects/app.md", category: "spec", name: "app.md", bytes: 20 },
        { relative_path: "commands/dw-run.md", category: "command", name: "dw-run.md", bytes: 30 },
      ]),
    ).toEqual({ state: 1, spec: 1, bugfix: 0, command: 1, rule: 0, support: 0 });
  });

  it("selects a viewer language from the artifact path", () => {
    expect(artifactLanguage("agent-registry.json")).toBe("json");
    expect(artifactLanguage("STATE.md")).toBe("markdown");
  });

  it("formats artifact sizes for compact lists", () => {
    expect(formatArtifactSize(512)).toBe("512 B");
    expect(formatArtifactSize(2048)).toBe("2.0 KB");
  });
});

describe("evidence model", () => {
  it("summarizes runs, items, submitted, and stale evidence", () => {
    expect(
      evidenceCounts([
        {
          id: "run:1",
          record_type: "run",
          run_id: 1,
          item_id: null,
          project_path: "/repo",
          status: "submitted",
          summary: "",
          kind: "run",
          title: "/dw-plan demo",
          created_at: "2026-05-20T00:00:00Z",
          completed_at: null,
          stale: false,
        },
        {
          id: "item:1",
          record_type: "item",
          run_id: null,
          item_id: 1,
          project_path: "/repo",
          status: "stale",
          summary: "",
          kind: "qa-report",
          title: "QA report",
          relative_path: "spec/prd-demo/QA/qa-report.md",
          created_at: "2026-05-20T00:00:00Z",
          completed_at: null,
          stale: true,
        },
      ]),
    ).toEqual({ runs: 1, items: 1, submitted: 1, stale: 1 });
  });

  it("normalizes evidence labels and manual link input", () => {
    expect(evidenceStatusLabel("submitted")).toBe("Submitted");
    expect(parseEvidenceLinks(" spec/a.md\n\n.dw/spec/b.md ")).toEqual([
      "spec/a.md",
      ".dw/spec/b.md",
    ]);
  });
});

describe("workspace model", () => {
  it("uses the current project path as the default workspace root", () => {
    expect(createDefaultWorkspaceRoot(" /repo/clia-app ")).toBe("/repo/clia-app");
  });

  it("falls back to the project path when a saved project has no display name", () => {
    expect(
      projectDisplayName({
        id: 1,
        workspace_id: 1,
        name: " ",
        path: "/repo/clia-app",
        remote_url: null,
        created_at: "2026-05-20T00:00:00Z",
      }),
    ).toBe("/repo/clia-app");
  });

  it("parses persisted selection ids and rejects junk", () => {
    expect(parseStateId("3")).toBe(3);
    expect(parseStateId(" 42 ")).toBe(42);
    expect(parseStateId(null)).toBeNull();
    expect(parseStateId("")).toBeNull();
    expect(parseStateId("abc")).toBeNull();
    expect(parseStateId("0")).toBeNull();
    expect(parseStateId("-1")).toBeNull();
  });

  it("restores the remembered workspace, falling back to the newest", () => {
    const workspaces = [
      { id: 2, name: "t1", root_path: "/tmp/t1", created_at: "2026-05-23T00:00:00Z" },
      { id: 1, name: "Local", root_path: "/repo", created_at: "2026-05-20T00:00:00Z" },
    ];
    expect(pickActiveWorkspace(workspaces, 1)?.id).toBe(1);
    expect(pickActiveWorkspace(workspaces, 99)?.id).toBe(2);
    expect(pickActiveWorkspace(workspaces, null)?.id).toBe(2);
    expect(pickActiveWorkspace([], 1)).toBeNull();
  });

  it("restores the remembered project, falling back to the first", () => {
    const projects = [
      {
        id: 5,
        workspace_id: 1,
        name: "alpha",
        path: "/repo/alpha",
        remote_url: null,
        created_at: "2026-05-20T00:00:00Z",
      },
      {
        id: 6,
        workspace_id: 1,
        name: "beta",
        path: "/repo/beta",
        remote_url: null,
        created_at: "2026-05-21T00:00:00Z",
      },
    ];
    expect(pickActiveProject(projects, 6)?.id).toBe(6);
    expect(pickActiveProject(projects, 99)?.id).toBe(5);
    expect(pickActiveProject(projects, null)?.id).toBe(5);
    expect(pickActiveProject([], 6)).toBeNull();
  });
});

describe("source model", () => {
  it("selects a viewer language from source paths", () => {
    expect(sourceLanguage("src/App.tsx")).toBe("javascript");
    expect(sourceLanguage("src/main.js")).toBe("javascript");
    expect(sourceLanguage("package.json")).toBe("json");
    expect(sourceLanguage("README.md")).toBe("markdown");
    expect(sourceLanguage("src-tauri/src/lib.rs")).toBe("rust");
    expect(sourceLanguage("src/styles.css")).toBe("css");
    expect(sourceLanguage("index.html")).toBe("html");
    expect(sourceLanguage("LICENSE")).toBe("plain");
  });

  it("formats source sizes for metadata", () => {
    expect(formatSourceSize(42)).toBe("42 B");
    expect(formatSourceSize(1536)).toBe("1.5 KB");
  });

  it("counts nested source files", () => {
    expect(
      countSourceFiles([
        {
          relative_path: "src",
          name: "src",
          kind: "directory",
          extension: null,
          bytes: null,
          children: [
            {
              relative_path: "src/App.tsx",
              name: "App.tsx",
              kind: "file",
              extension: "tsx",
              bytes: 100,
              children: [],
            },
          ],
        },
        {
          relative_path: "README.md",
          name: "README.md",
          kind: "file",
          extension: "md",
          bytes: 50,
          children: [],
        },
      ]),
    ).toBe(2);
  });
});

describe("patch review model", () => {
  const files = [
    {
      path: "src/App.tsx",
      old_path: null,
      status: "M",
      area: "unstaged" as const,
      additions: 5,
      deletions: 2,
      can_stage_hunks: true,
    },
    {
      path: "README.md",
      old_path: null,
      status: "M",
      area: "staged" as const,
      additions: 1,
      deletions: 0,
      can_stage_hunks: true,
    },
    {
      path: "new.txt",
      old_path: null,
      status: "??",
      area: "unstaged" as const,
      additions: 0,
      deletions: 0,
      can_stage_hunks: false,
    },
  ];

  it("groups changed files by patch area", () => {
    expect(groupChangedFiles(files)).toEqual({
      staged: [files[1]],
      unstaged: [files[0], files[2]],
    });
  });

  it("counts staged and unstaged files", () => {
    expect(countPatchAreas(files)).toEqual({ staged: 1, unstaged: 2 });
  });

  it("formats stats and action labels", () => {
    expect(formatPatchStats(files[0])).toBe("+5 -2");
    expect(fileActionLabel(files[0])).toBe("Stage file");
    expect(fileActionLabel(files[1])).toBe("Unstage file");
  });

  it("keeps hunk actions unavailable for untracked files", () => {
    expect(canStageHunks(files[0])).toBe(true);
    expect(canStageHunks(files[2])).toBe(false);
    expect(canStageHunks(null)).toBe(false);
  });
});

describe("terminal model", () => {
  const session = {
    id: "terminal-1",
    title: "Terminal 1",
    cwd: "/repo",
    shell: "bash",
    status: "running" as const,
    log_path: "/tmp/clia-app/terminal/terminal-terminal-1.log",
    created_at: "2026-05-20T00:00:00Z",
    updated_at: "2026-05-20T00:00:00Z",
    exit_code: null,
  };

  it("identifies running sessions and labels statuses", () => {
    expect(isTerminalRunning(session)).toBe(true);
    expect(terminalStatusLabel("stopped")).toBe("Stopped");
  });

  it("appends output to the matching session buffer", () => {
    expect(
      appendTerminalOutput({ "terminal-1": "hello" }, { session_id: "terminal-1", data: " world" }),
    ).toEqual({ "terminal-1": "hello world" });
  });

  it("bounds terminal buffers to the configured scrollback", () => {
    const data = Array.from(
      { length: TERMINAL_SCROLLBACK_LINES + 2 },
      (_, index) => `${index}`,
    ).join("\n");
    const buffers = appendTerminalOutput({}, { session_id: "terminal-1", data });

    expect(buffers["terminal-1"].split("\n")).toHaveLength(TERMINAL_SCROLLBACK_LINES);
  });

  it("applies status events to the matching session", () => {
    expect(
      applyTerminalStatus([session], {
        session_id: "terminal-1",
        status: "exited",
        exit_code: 0,
      })[0],
    ).toMatchObject({ status: "exited", exit_code: 0 });
  });

  it("upserts terminal sessions", () => {
    expect(upsertTerminalSession([], session)).toEqual([session]);
    expect(upsertTerminalSession([session], { ...session, title: "Renamed" })[0].title).toBe(
      "Renamed",
    );
  });

  it("finds workspace terminal sessions by normalized cwd", () => {
    const stopped = {
      ...session,
      id: "terminal-2",
      cwd: "/workspace/",
      status: "stopped" as const,
    };
    const running = { ...session, id: "terminal-3", cwd: "/workspace" };

    expect(normalizeComparablePath("/workspace/")).toBe("/workspace");
    expect(findTerminalForPath([stopped, running], "/workspace")?.id).toBe("terminal-2");
    expect(findTerminalForPath([stopped, running], "/workspace/", true)?.id).toBe("terminal-3");
  });
});

describe("agent model", () => {
  const session = {
    id: 1,
    profile_id: 1,
    workspace_id: 1,
    project_id: 1,
    requirement_card_id: null,
    scope: "chat",
    project_path: "/repo",
    provider: "codex",
    model: null,
    reasoning_effort: null,
    sandbox: "read-only",
    context_mode: "auto_lean",
    provider_session_id: null,
    codex_session_id: null,
    status: "running",
    title: "Brainstorm",
    created_at: "2026-05-20T00:00:00Z",
    updated_at: "2026-05-20T00:00:00Z",
  };

  it("identifies running agents and labels statuses", () => {
    expect(isAgentRunning(session)).toBe(true);
    expect(hasRunningAgentSession([session])).toBe(true);
    expect(hasRunningAgentSession([{ ...session, status: "stopped" }])).toBe(false);
    expect(agentStatusLabel("running")).toBe("Working");
    expect(agentStatusLabel("stopped")).toBe("Stopped");
  });

  it("upserts agent sessions newest first", () => {
    const older = { ...session, id: 2, updated_at: "2026-05-19T00:00:00Z" };
    const renamed = { ...session, title: "Renamed", updated_at: "2026-05-21T00:00:00Z" };

    expect(upsertAgentSession([], session)).toEqual([session]);
    expect(upsertAgentSession([session, older], renamed)[0].title).toBe("Renamed");
  });
});
