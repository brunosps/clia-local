import { describe, expect, it } from "vitest";
import {
  BUILTIN_PRESETS,
  loadFlowRegistry,
  parseFlowIndex,
  PRESET_SPEC_KIT,
  recommendFlow,
} from "./flows";
import { buildPhaseCommand, parseWorkbenchSchema, phaseById } from "./workbenchSchema";

type Reader = (relativePath: string) => Promise<
  { ok: true; value: string } | { ok: false; error: string }
>;

function readerFrom(files: Record<string, string>): Reader {
  return async (relativePath) =>
    relativePath in files
      ? { ok: true, value: files[relativePath] }
      : { ok: false, error: "not found" };
}

describe("parseFlowIndex", () => {
  it("returns null for invalid or empty payloads", () => {
    expect(parseFlowIndex("not json")).toBeNull();
    expect(parseFlowIndex(JSON.stringify({ flows: [] }))).toBeNull();
    expect(parseFlowIndex(JSON.stringify({}))).toBeNull();
  });

  it("keeps the first of duplicate ids and defaults the label to the id", () => {
    const index = parseFlowIndex(
      JSON.stringify({ flows: [{ id: "a" }, { id: "a", label: "dup" }, { id: "b", label: "B" }] }),
    );
    expect(index?.flows.map((f) => f.id)).toEqual(["a", "b"]);
    expect(index?.flows[0].label).toBe("a");
    expect(index?.flows[1].label).toBe("B");
  });
});

describe("loadFlowRegistry", () => {
  it("falls back to a single dev-workflow flow when there is no index", async () => {
    const registry = await loadFlowRegistry(readerFrom({}));
    expect(registry.flows.map((f) => f.id)).toEqual(["dev-workflow"]);
    expect(registry.schemas["dev-workflow"].phases.length).toBeGreaterThan(0);
  });

  it("loads each listed flow file and honors the default", async () => {
    const registry = await loadFlowRegistry(
      readerFrom({
        "flows/index.json": JSON.stringify({
          flows: [
            { id: "dev-workflow", label: "dev-workflow" },
            { id: "spec-kit", label: "GitHub spec-kit" },
          ],
          default: "spec-kit",
        }),
        "flows/dev-workflow.json": JSON.stringify({
          version: 1,
          phases: [{ id: "backlog", label: "Backlog", status: "draft", action: { type: "none" } }],
        }),
        "flows/spec-kit.json": JSON.stringify(PRESET_SPEC_KIT),
      }),
    );
    expect(registry.flows.map((f) => f.id)).toEqual(["dev-workflow", "spec-kit"]);
    expect(registry.defaultFlowId).toBe("spec-kit");
  });

  it("backfills analyze/suggest commands from the bundled preset when the index omits them", async () => {
    const registry = await loadFlowRegistry(
      readerFrom({
        "flows/index.json": JSON.stringify({
          flows: [{ id: "dev-workflow", label: "dev-workflow", preset: "dev-workflow" }],
        }),
      }),
    );
    const flow = registry.flows.find((f) => f.id === "dev-workflow");
    expect(flow?.analyzeCommand).toBe("/dw-analyze-project");
    expect(flow?.suggestCommand).toBe("/dw-opportunities");
  });

  it("falls back to a matching built-in preset when a flow file is missing", async () => {
    const registry = await loadFlowRegistry(
      readerFrom({
        "flows/index.json": JSON.stringify({
          flows: [{ id: "spec-kit", label: "GitHub spec-kit", preset: "spec-kit" }],
        }),
      }),
    );
    expect(registry.schemas["spec-kit"]).toBeDefined();
    expect(phaseById(registry.schemas["spec-kit"].phases, "sk-specify")).toBeDefined();
  });
});

describe("recommendFlow", () => {
  it("recommends the flow whose vocabulary overlaps the card text", async () => {
    const registry = await loadFlowRegistry(
      readerFrom({
        "flows/index.json": JSON.stringify({
          flows: [
            { id: "dev-workflow", label: "dev-workflow", preset: "dev-workflow" },
            { id: "spec-kit", label: "GitHub spec-kit", preset: "spec-kit" },
          ],
        }),
      }),
    );
    const rec = recommendFlow(
      { title: "Specify and implement the OAuth feature", body: "clarify scope first" },
      registry,
    );
    expect(rec?.flowId).toBe("spec-kit");
    expect(rec?.score).toBeGreaterThan(0);
  });

  it("returns null when the card has no meaningful tokens", async () => {
    const registry = await loadFlowRegistry(readerFrom({}));
    expect(recommendFlow({ title: "", body: "" }, registry)).toBeNull();
  });
});

describe("built-in presets", () => {
  it("are all valid schemas with at least one phase and round-trip through JSON", () => {
    for (const preset of Object.values(BUILTIN_PRESETS)) {
      const parsed = parseWorkbenchSchema(JSON.stringify(preset.schema));
      expect(parsed.usedDefault, `${preset.meta.id} should be valid`).toBe(false);
      expect(parsed.schema.phases.length).toBeGreaterThan(0);
    }
  });

  it("register the four expected ids", () => {
    expect(Object.keys(BUILTIN_PRESETS).sort()).toEqual(
      ["bmad", "dev-workflow", "openspec", "spec-kit"].sort(),
    );
  });
});

describe("spec-kit preset is framework-agnostic", () => {
  it("emits a /speckit.specify command from the card title", () => {
    const phase = phaseById(PRESET_SPEC_KIT.phases, "sk-specify")!;
    const command = buildPhaseCommand(phase, {
      title: "Add OAuth login",
      slug: "add-oauth-login",
      prd_slug: null,
      public_id: "CARD-1",
      id: 1,
    });
    expect(command).toBe('/speckit.specify "Add OAuth login"');
  });
});
