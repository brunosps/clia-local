/// <reference types="node" />

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

function readJsonFile<T>(path: string): T {
  return JSON.parse(readFileSync(path, "utf8")) as T;
}

describe("app version metadata", () => {
  it("keeps npm metadata aligned with the Tauri app version", () => {
    const root = process.cwd();
    const packageJson = readJsonFile<{ version: string }>(join(root, "package.json"));
    const tauriConfig = readJsonFile<{ version: string }>(
      join(root, "src-tauri", "tauri.conf.json"),
    );

    expect(packageJson.version).toBe(tauriConfig.version);
  });
});
