import { describe, expect, it } from "vitest";
import {
  clampNumberPreference,
  normalizeHexColorPreference,
  normalizeWorkspaceAccentColor,
  parseGitWorkbenchPreference,
  parseSourceWorkspacePreference,
  parseTabPreference,
  parseThemePreference,
  serializeSourceWorkspacePreference,
  workspaceAccentCssVariables,
} from "./uiPreferences";

describe("uiPreferences", () => {
  it("clamps numeric preferences", () => {
    expect(clampNumberPreference("640", 320, { min: 200, max: 620 })).toBe(620);
    expect(clampNumberPreference("180", 320, { min: 200, max: 620 })).toBe(200);
    expect(clampNumberPreference("bad", 320, { min: 200, max: 620 })).toBe(320);
  });

  it("parses only known app tabs", () => {
    expect(parseTabPreference("queue")).toBe("queue");
    expect(parseTabPreference("code")).toBe("code");
    expect(parseTabPreference("deploy")).toBe("deploy");
    expect(parseTabPreference("settings")).toBe("settings");
    expect(parseTabPreference("workbench")).toBe("queue");
    expect(parseTabPreference("machines")).toBe("deploy");
    expect(parseTabPreference("flows")).toBe("deploy");
    expect(parseTabPreference("cloud")).toBe("settings");
    expect(parseTabPreference("unknown")).toBeNull();
    expect(parseTabPreference(null)).toBeNull();
  });

  it("parses theme preference with clia as fallback", () => {
    expect(parseThemePreference("clia")).toBe("clia");
    expect(parseThemePreference("black")).toBe("black");
    expect(parseThemePreference("unknown")).toBe("clia");
    expect(parseThemePreference(null)).toBe("clia");
  });

  it("normalizes workspace accent colors", () => {
    expect(normalizeHexColorPreference("4F8CFF")).toBe("#4f8cff");
    expect(normalizeHexColorPreference("#2aaec8")).toBe("#2aaec8");
    expect(normalizeHexColorPreference("#bad")).toBeNull();
    expect(normalizeWorkspaceAccentColor("#4f8cff")).toBe("#4f8cff");
    expect(normalizeWorkspaceAccentColor("#010203")).toBeNull();
    expect(workspaceAccentCssVariables("#4f8cff")).toEqual({
      "--workspace-accent": "#4f8cff",
      "--workspace-accent-rgb": "79, 140, 255",
      "--workspace-accent-shell": "rgba(79, 140, 255, 0.06)",
      "--workspace-accent-soft": "rgba(79, 140, 255, 0.14)",
      "--workspace-accent-surface": "rgba(79, 140, 255, 0.18)",
      "--workspace-accent-surface-strong": "rgba(79, 140, 255, 0.26)",
      "--workspace-accent-muted": "rgba(79, 140, 255, 0.08)",
      "--workspace-accent-border": "rgba(79, 140, 255, 0.5)",
      "--workspace-accent-ring": "rgba(79, 140, 255, 0.78)",
      "--workspace-accent-text": "color-mix(in srgb, #4f8cff 76%, #ffffff)",
      "--workspace-accent-panel": "color-mix(in srgb, #151923 88%, #4f8cff)",
      "--workspace-accent-panel-strong": "color-mix(in srgb, #202633 76%, #4f8cff)",
      "--workspace-accent-button": "color-mix(in srgb, #4f8cff 42%, #d8e8ff)",
      "--workspace-accent-on": "#07111f",
    });
  });

  it("normalizes source workspace preference", () => {
    const parsed = parseSourceWorkspacePreference(
      JSON.stringify({
        openPaths: ["src/App.tsx", "src/App.tsx", "", 42, "src/main.tsx"],
        expandedPaths: ["src", "src", "", "src/source"],
        activePath: "src/main.tsx",
        sideTab: "search",
        preview: true,
        showHistory: true,
      }),
    );

    expect(parsed).toEqual({
      openPaths: ["src/App.tsx", "src/main.tsx"],
      expandedPaths: ["src", "src/source"],
      activePath: "src/main.tsx",
      sideTab: "search",
      preview: true,
      showHistory: true,
    });
    expect(parseSourceWorkspacePreference(serializeSourceWorkspacePreference(parsed))).toEqual(
      parsed,
    );
  });

  it("falls back safely on malformed git preferences", () => {
    expect(parseGitWorkbenchPreference("{bad json")).toEqual({
      view: "commits",
      includeRemotes: false,
      includeTags: false,
      limit: 200,
      detailTab: "commit",
    });
    expect(
      parseGitWorkbenchPreference(
        JSON.stringify({
          view: "local",
          includeRemotes: true,
          includeTags: true,
          limit: 5000,
          detailTab: "changes",
        }),
      ),
    ).toEqual({
      view: "local",
      includeRemotes: true,
      includeTags: true,
      limit: 2000,
      detailTab: "changes",
    });
  });
});
