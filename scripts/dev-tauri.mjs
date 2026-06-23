/* global process */

import { spawn } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";

function isWsl() {
  if (process.platform !== "linux") return false;
  if (process.env.WSL_DISTRO_NAME || process.env.WSL_INTEROP) return true;
  try {
    return /microsoft|wsl/i.test(readFileSync("/proc/version", "utf8"));
  } catch {
    return existsSync("/mnt/wslg");
  }
}

const env = { ...process.env };

if (isWsl()) {
  env.WEBKIT_DISABLE_COMPOSITING_MODE ??= "1";
  env.WEBKIT_DISABLE_DMABUF_RENDERER ??= "1";
  env.LIBGL_ALWAYS_SOFTWARE ??= "1";
  env.MESA_LOADER_DRIVER_OVERRIDE ??= "llvmpipe";
}

const child = spawn("corepack", ["pnpm", "exec", "tauri", "dev"], {
  env,
  shell: process.platform === "win32",
  stdio: "inherit",
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code ?? 1);
});
