#!/usr/bin/env node
/* global process, console */
// Install a `clia-local` launcher so a folder can be opened straight from a shell:
//
//   clia-local /home/bruno/code/clia-remote
//
// Writes a wrapper into ~/.local/bin (forwarding "$@" to the real binary) plus a
// .desktop entry with `Exec=... %f`, so file managers can "Open with clia.local"
// on a folder too. There is no single-instance plugin, so two folders open as two
// independent windows — same mental model as separate editor windows.
//
// Usage:
//   node scripts/install-launcher.mjs [--bin /path/to/clia-local]
//
// Without --bin it looks for a release build, then a dev build.

import { chmodSync, existsSync, mkdirSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const home = homedir();

function resolveBinary() {
  const flag = process.argv.indexOf("--bin");
  if (flag !== -1) {
    const explicit = process.argv[flag + 1];
    if (!explicit) fail("--bin needs a path");
    const resolved = path.resolve(explicit);
    if (!existsSync(resolved)) fail(`binary not found: ${resolved}`);
    return resolved;
  }
  const candidates = [
    path.join(repoRoot, "src-tauri/target/release/clia-local"),
    path.join(repoRoot, "src-tauri/target/debug/clia-local"),
  ];
  const found = candidates.find((candidate) => existsSync(candidate));
  if (!found) {
    fail(
      `no clia-local binary found. Build one first (corepack pnpm build) or pass --bin <path>.\nLooked in:\n  ${candidates.join("\n  ")}`,
    );
  }
  return found;
}

function fail(message) {
  console.error(`install-launcher: ${message}`);
  process.exit(1);
}

const binary = resolveBinary();
const binDir = path.join(home, ".local/bin");
const launcher = path.join(binDir, "clia-local");
const desktopDir = path.join(home, ".local/share/applications");
const desktopFile = path.join(desktopDir, "clia-local.desktop");

mkdirSync(binDir, { recursive: true });
mkdirSync(desktopDir, { recursive: true });

// `exec` so signals and the exit code belong to the app, not the wrapper. The path
// is quoted because the repo may live under a directory with spaces.
writeFileSync(launcher, `#!/bin/sh\nexec "${binary}" "$@"\n`, { mode: 0o755 });
chmodSync(launcher, 0o755);

writeFileSync(
  desktopFile,
  [
    "[Desktop Entry]",
    "Type=Application",
    "Name=clia.local",
    "Comment=Fully local agent development environment",
    `Exec=${launcher} %f`,
    "Icon=" + path.join(repoRoot, "src-tauri/icons/128x128.png"),
    "Terminal=false",
    "Categories=Development;IDE;",
    "MimeType=inode/directory;",
    "",
  ].join("\n"),
);

console.log(`clia-local -> ${binary}`);
console.log(`  launcher: ${launcher}`);
console.log(`  desktop:  ${desktopFile}`);
if (!(process.env.PATH ?? "").split(path.delimiter).includes(binDir)) {
  console.log(`\nNote: ${binDir} is not in your PATH. Add it to use \`clia-local\` directly.`);
}
