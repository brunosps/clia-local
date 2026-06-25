/* global URL, process, console */
// Copies the winbox-gui CLI binary into src-tauri/binaries/winbox/ so it can be
// bundled as a Tauri resource (mirrors scripts/prepare-rtk.mjs). The binary is the
// `winbox-gui` release build, which speaks the `--json` envelope contract the
// Machines/Deploy panels expect.
//
// Source resolution order:
//   1. $WINBOX_GUI_BIN (explicit path to the built winbox-gui binary)
//   2. ../winbox-gui/src-tauri/target/release/winbox-gui   (sibling checkout)
//   3. ~/code/winbox-gui/src-tauri/target/release/winbox-gui
import { chmod, copyFile, mkdir, stat } from "node:fs/promises";
import { homedir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const isWin = process.platform === "win32";
const exeName = isWin ? "winbox.exe" : "winbox";
const srcName = isWin ? "winbox-gui.exe" : "winbox-gui";

const root = fileURLToPath(new URL("..", import.meta.url));
const outputDir = join(root, "src-tauri", "binaries", "winbox");
const outputPath = join(outputDir, exeName);

const candidates = [
  process.env.WINBOX_GUI_BIN,
  join(root, "..", "winbox-gui", "src-tauri", "target", "release", srcName),
  join(homedir(), "code", "winbox-gui", "src-tauri", "target", "release", srcName),
].filter(Boolean);

let source = null;
for (const candidate of candidates) {
  try {
    if ((await stat(candidate)).isFile()) {
      source = candidate;
      break;
    }
  } catch {
    /* not here, try next */
  }
}

if (!source) {
  console.error(
    "Could not find the winbox-gui binary. Build it first " +
      "(cargo build --release --manifest-path <winbox-gui>/src-tauri/Cargo.toml) " +
      "or set WINBOX_GUI_BIN to its path. Looked in:\n  " +
      candidates.join("\n  "),
  );
  process.exit(1);
}

await mkdir(outputDir, { recursive: true });
await copyFile(source, outputPath);
if (!isWin) await chmod(outputPath, 0o755);
console.log(`Prepared ${exeName} at ${outputPath} (from ${source})`);
