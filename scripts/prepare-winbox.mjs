/* global URL, process, console, fetch */
// Places the winbox CLI binary into src-tauri/binaries/winbox/ so it can be bundled
// as a Tauri resource (mirrors scripts/prepare-rtk.mjs). The binary is the
// `winbox-gui` release build, which speaks the `--json` envelope contract the
// Machines/Deploy panels expect.
//
// Strategy:
//   1. If scripts/winbox-release.json has an asset for this platform, DOWNLOAD it from
//      the published winbox-gui release and verify the sha256 (reproducible, CI-friendly).
//   2. Otherwise (no manifest entry, offline, or download fails), FALL BACK to copying a
//      locally built winbox-gui binary ($WINBOX_GUI_BIN or a sibling/~code checkout).
import { createHash } from "node:crypto";
import { createWriteStream } from "node:fs";
import { chmod, copyFile, mkdir, readFile, readdir, rm, stat } from "node:fs/promises";
import { homedir, tmpdir } from "node:os";
import { basename, join } from "node:path";
import { fileURLToPath } from "node:url";
import { pipeline } from "node:stream/promises";
import { spawnSync } from "node:child_process";

const isWin = process.platform === "win32";
const exeName = isWin ? "winbox.exe" : "winbox";
const srcBinName = isWin ? "winbox-gui.exe" : "winbox-gui";

const root = fileURLToPath(new URL("..", import.meta.url));
const outputDir = join(root, "src-tauri", "binaries", "winbox");
const outputPath = join(outputDir, exeName);

await mkdir(outputDir, { recursive: true });

if (await tryDownload()) {
  // done
} else if (await tryLocalCopy()) {
  // done
} else {
  console.error(
    "Could not obtain the winbox CLI. Publish a winbox-gui release (so it can be " +
      "downloaded), set WINBOX_GUI_BIN to a built binary, or keep a winbox-gui checkout " +
      "next to this repo (cargo build --release).",
  );
  process.exit(1);
}

async function tryDownload() {
  let manifest;
  try {
    manifest = JSON.parse(await readFile(new URL("winbox-release.json", import.meta.url), "utf8"));
  } catch {
    return false; // no manifest yet
  }
  const key = `${process.platform}-${process.arch}`;
  const asset = manifest.assets?.[key];
  if (!asset) {
    console.warn(`winbox-release.json has no asset for ${key}; falling back to local build.`);
    return false;
  }
  const downloadDir = join(tmpdir(), `clia-winbox-${manifest.version}-${process.pid}`);
  const archivePath = join(downloadDir, asset.file);
  const extractDir = join(downloadDir, "extract");
  try {
    await mkdir(extractDir, { recursive: true });
    console.log(`Downloading winbox ${manifest.version} for ${key}`);
    await downloadAsset(manifest, asset, downloadDir, archivePath);

    const actual = createHash("sha256")
      .update(await readFile(archivePath))
      .digest("hex");
    if (actual !== asset.sha256) {
      throw new Error(`checksum mismatch for ${asset.file}: expected ${asset.sha256}, got ${actual}`);
    }

    extractArchive(archivePath, extractDir);
    const binary = await findBinary(extractDir, srcBinName);
    if (!binary) throw new Error(`could not find ${srcBinName} in ${asset.file}`);

    await copyFile(binary, outputPath);
    if (!isWin) await chmod(outputPath, 0o755);
    console.log(`Prepared ${exeName} at ${outputPath} (downloaded ${asset.file})`);
    return true;
  } catch (error) {
    console.warn(`winbox download failed (${error.message}); falling back to local build.`);
    return false;
  } finally {
    await rm(downloadDir, { recursive: true, force: true });
  }
}

async function tryLocalCopy() {
  const candidates = [
    process.env.WINBOX_GUI_BIN,
    join(root, "..", "winbox-gui", "src-tauri", "target", "release", srcBinName),
    join(homedir(), "code", "winbox-gui", "src-tauri", "target", "release", srcBinName),
  ].filter(Boolean);
  for (const candidate of candidates) {
    try {
      if ((await stat(candidate)).isFile()) {
        await copyFile(candidate, outputPath);
        if (!isWin) await chmod(outputPath, 0o755);
        console.log(`Prepared ${exeName} at ${outputPath} (from local build ${candidate})`);
        return true;
      }
    } catch {
      /* try next */
    }
  }
  return false;
}

function hasGh() {
  return spawnSync("gh", ["--version"], { stdio: "ignore" }).status === 0;
}

// Fetch the release asset. Prefer `gh` (authenticated → works for private repos too);
// fall back to the direct download URL for public releases.
async function downloadAsset(manifest, asset, downloadDir, archivePath) {
  if (manifest.repo && manifest.tag && hasGh()) {
    const r = spawnSync(
      "gh",
      ["release", "download", manifest.tag, "--repo", manifest.repo, "--pattern", asset.file, "--dir", downloadDir, "--clobber"],
      { stdio: "inherit" },
    );
    if (r.status === 0) return;
    console.warn("gh release download failed; trying the direct URL.");
  }
  const url = `${manifest.baseUrl}/${asset.file}`;
  const response = await fetch(url);
  if (!response.ok || !response.body) throw new Error(`download ${url}: ${response.status}`);
  await pipeline(response.body, createWriteStream(archivePath));
}

function extractArchive(archivePath, extractDir) {
  if (archivePath.endsWith(".zip")) {
    const unzip = spawnSync("unzip", ["-q", archivePath, "-d", extractDir], { stdio: "inherit" });
    if (unzip.status !== 0) {
      const ps = spawnSync(
        "powershell.exe",
        ["-NoProfile", "-Command", `Expand-Archive -Force '${archivePath}' '${extractDir}'`],
        { stdio: "inherit" },
      );
      if (ps.status !== 0) throw new Error(`failed to extract ${basename(archivePath)}`);
    }
  } else {
    const tar = spawnSync("tar", ["-xzf", archivePath, "-C", extractDir], { stdio: "inherit" });
    if (tar.status !== 0) throw new Error(`failed to extract ${basename(archivePath)}`);
  }
}

async function findBinary(dir, name) {
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isFile() && entry.name === name) return path;
    if (entry.isDirectory()) {
      const found = await findBinary(path, name);
      if (found) return found;
    }
  }
  return null;
}
