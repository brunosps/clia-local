import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({ setZoom: vi.fn() }),
}));

import { APP_ZOOM_STEPS, DEFAULT_APP_ZOOM, normalizeAppZoom, stepAppZoom } from "./appZoom";

describe("app zoom", () => {
  it("clamps stored values into the supported range", () => {
    expect(normalizeAppZoom(125)).toBe(125);
    expect(normalizeAppZoom("125")).toBe(125);
    expect(normalizeAppZoom(117.4)).toBe(117);
    expect(normalizeAppZoom(10)).toBe(APP_ZOOM_STEPS[0]);
    expect(normalizeAppZoom(1000)).toBe(APP_ZOOM_STEPS[APP_ZOOM_STEPS.length - 1]);
  });

  it("falls back to the default for values that are not numbers", () => {
    expect(normalizeAppZoom(null)).toBe(DEFAULT_APP_ZOOM);
    expect(normalizeAppZoom(undefined)).toBe(DEFAULT_APP_ZOOM);
    expect(normalizeAppZoom("abc")).toBe(DEFAULT_APP_ZOOM);
    expect(normalizeAppZoom("")).toBe(DEFAULT_APP_ZOOM);
  });

  it("steps to the next and previous preset", () => {
    expect(stepAppZoom(100, 1)).toBe(110);
    expect(stepAppZoom(100, -1)).toBe(90);
    expect(stepAppZoom(125, 1)).toBe(150);
    expect(stepAppZoom(125, -1)).toBe(110);
  });

  it("stops at the ends instead of wrapping around", () => {
    const min = APP_ZOOM_STEPS[0];
    const max = APP_ZOOM_STEPS[APP_ZOOM_STEPS.length - 1];
    expect(stepAppZoom(min, -1)).toBe(min);
    expect(stepAppZoom(max, 1)).toBe(max);
  });

  it("steps from a value that is not itself a preset", () => {
    expect(stepAppZoom(117, 1)).toBe(125);
    expect(stepAppZoom(117, -1)).toBe(110);
  });
});
