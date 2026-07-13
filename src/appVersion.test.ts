/// <reference types="node" />

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

function readJsonFile<T>(path: string): T {
  return JSON.parse(readFileSync(path, "utf8")) as T;
}

function readCargoPackageVersion(path: string): string {
  const content = readFileSync(path, "utf8");
  const match = content.match(/^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m);
  if (!match) throw new Error(`Could not read Cargo package version from ${path}`);
  return match[1];
}

describe("app version metadata", () => {
  it("keeps npm, Tauri, and Cargo metadata aligned", () => {
    const root = process.cwd();
    const packageJson = readJsonFile<{ version: string }>(join(root, "package.json"));
    const tauriConfig = readJsonFile<{ version: string }>(
      join(root, "src-tauri", "tauri.conf.json"),
    );
    const cargoVersion = readCargoPackageVersion(join(root, "src-tauri", "Cargo.toml"));

    expect(packageJson.version).toBe(tauriConfig.version);
    expect(packageJson.version).toBe(cargoVersion);
  });
});
