import {
  readText as readNativeClipboardText,
  writeText as writeNativeClipboardText,
} from "@tauri-apps/plugin-clipboard-manager";

import { api } from "./tauri";

/**
 * Clipboard text access with the WSL escape hatches layered in.
 *
 * Three routes exist and none of them works everywhere:
 *   - `navigator.clipboard` — blocked by WebKitGTK in some builds (no user
 *     gesture / non-secure context), and silently rejects rather than throwing
 *     synchronously.
 *   - the Tauri clipboard plugin (arboard) — talks to the X11/Wayland selection
 *     the app is actually connected to.
 *   - `powershell.exe` / `clip.exe` — only under WSL, where WSLg does not always
 *     bridge the Windows selection to the Linux one.
 *
 * So both helpers try in order and report whether *any* route worked, instead of
 * assuming the first one did.
 */

export async function writeClipboardText(text: string): Promise<boolean> {
  let wrote = false;
  try {
    await writeNativeClipboardText(text);
    wrote = true;
  } catch {
    // Fall through to the DOM API below.
  }
  if (!wrote) {
    try {
      await navigator.clipboard?.writeText(text);
      wrote = true;
    } catch {
      // Both in-process routes failed; WSL may still work.
    }
  }
  // Also push to the Windows clipboard when running under WSL, so the text is
  // available to Windows apps. No-op (false) off WSL — never an error path.
  const bridged = await api.writeWindowsClipboardText(text);
  return wrote || (bridged.ok && bridged.value);
}

/**
 * Read clipboard text, falling back to the Windows clipboard on WSL.
 * Returns null when every route comes back empty.
 */
export async function readClipboardText(): Promise<string | null> {
  try {
    const native = await readNativeClipboardText();
    if (native?.trim()) return native;
  } catch {
    // Ignore and try the next route.
  }
  try {
    const dom = await navigator.clipboard?.readText();
    if (dom?.trim()) return dom;
  } catch {
    // Ignore and try the next route.
  }
  const windows = await api.readWindowsClipboardText();
  return windows.ok && windows.value?.trim() ? windows.value : null;
}
