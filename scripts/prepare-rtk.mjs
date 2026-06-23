/* global URL, process, console, fetch */
import { createHash } from "node:crypto";
import { createWriteStream } from "node:fs";
import { chmod, copyFile, mkdir, readFile, readdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { pipeline } from "node:stream/promises";
import { spawnSync } from "node:child_process";

const root = new URL("..", import.meta.url);
const manifest = JSON.parse(await readFile(new URL("rtk-release.json", import.meta.url), "utf8"));
const key = `${process.platform}-${process.arch}`;
const asset = manifest.assets[key];

if (!asset) {
  throw new Error(`No RTK asset configured for ${key}`);
}

const url = `${manifest.baseUrl}/${asset.file}`;
const downloadDir = join(tmpdir(), `clia-rtk-${manifest.version}-${Date.now()}`);
const archivePath = join(downloadDir, asset.file);
const extractDir = join(downloadDir, "extract");
const outputDir = new URL("src-tauri/binaries/rtk/", root);
const outputName = process.platform === "win32" ? "rtk.exe" : "rtk";
const outputPath = new URL(outputName, outputDir);

await mkdir(downloadDir, { recursive: true });
await mkdir(extractDir, { recursive: true });
await mkdir(outputDir, { recursive: true });

console.log(`Downloading RTK ${manifest.version} for ${key}`);
const response = await fetch(url);
if (!response.ok || !response.body) {
  throw new Error(`Failed to download ${url}: ${response.status}`);
}
await pipeline(response.body, createWriteStream(archivePath));

const hash = createHash("sha256");
hash.update(await readFile(archivePath));
const actual = hash.digest("hex");
if (actual !== asset.sha256) {
  throw new Error(`RTK checksum mismatch for ${asset.file}: expected ${asset.sha256}, got ${actual}`);
}

if (asset.file.endsWith(".zip")) {
  const unzip = spawnSync("unzip", ["-q", archivePath, "-d", extractDir], { stdio: "inherit" });
  if (unzip.status !== 0) {
    const powershell = spawnSync(
      "powershell.exe",
      ["-NoProfile", "-Command", `Expand-Archive -Force '${archivePath}' '${extractDir}'`],
      { stdio: "inherit" },
    );
    if (powershell.status !== 0) throw new Error(`Failed to extract ${basename(archivePath)}`);
  }
} else {
  const tar = spawnSync("tar", ["-xzf", archivePath, "-C", extractDir], { stdio: "inherit" });
  if (tar.status !== 0) throw new Error(`Failed to extract ${basename(archivePath)}`);
}

const binary = await findBinary(extractDir, outputName);
if (!binary) {
  throw new Error(`Could not find ${outputName} in ${asset.file}`);
}

await copyFile(binary, outputPath);
if (process.platform !== "win32") await chmod(outputPath, 0o755);
await rm(downloadDir, { recursive: true, force: true });
console.log(`Prepared ${outputName} at ${outputPath.pathname}`);

async function findBinary(dir, name) {
  const entries = await readdir(dir, { withFileTypes: true });
  for (const entry of entries) {
    const path = join(dir, entry.name);
    if (entry.isFile() && entry.name === name) return path;
    if (entry.isDirectory()) {
      const found = await findBinary(path, name);
      if (found) return found;
    }
  }
  return null;
}
