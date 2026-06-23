import { describe, expect, it, vi } from "vitest";

vi.mock("./tauri", () => ({
  api: {
    getAppState: vi.fn(),
    setAppState: vi.fn(),
  },
}));

import { DEFAULT_LOCALE, normalizeLocale, translate } from "./i18n";

describe("i18n", () => {
  it("normalizes unsupported locale values to the default locale", () => {
    expect(normalizeLocale("en")).toBe("en");
    expect(normalizeLocale("pt-BR")).toBe("pt-BR");
    expect(normalizeLocale("pt")).toBe(DEFAULT_LOCALE);
    expect(normalizeLocale(null)).toBe(DEFAULT_LOCALE);
  });

  it("translates and interpolates known keys", () => {
    expect(translate("en", "app.name")).toBe("clia.dev");
    expect(translate("pt-BR", "topbar.archived", { count: 7 })).toBe("Archived (7)");
    expect(translate("en", "knowledge.removeSource", { name: "brief.pdf" })).toBe(
      "Remove brief.pdf",
    );
  });

  it("keeps product and development keywords stable in pt-BR", () => {
    expect(translate("pt-BR", "nav.knowledge")).toBe("Knowledge base");
    expect(translate("pt-BR", "welcome.title")).toBe("Agent development workspace");
    expect(translate("pt-BR", "skills.scope")).toBe("Capabilities");
  });
});
