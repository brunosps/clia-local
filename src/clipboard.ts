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
 * Input types a paste can land in. Restricted to the types that support
 * `setSelectionRange` — `email` and `number` throw InvalidStateError on it, so
 * they are deliberately absent rather than handled.
 */
const TEXT_INPUT_TYPES = new Set(["text", "search", "url", "tel", "password", ""]);

/**
 * Write into a controlled React field.
 *
 * Assigning `.value` directly is invisible to React — it tracks the previous value
 * on the node and would skip the change. Going through the prototype's setter and
 * then dispatching `input` is what makes React pick the new value up.
 */
function insertIntoField(field: HTMLInputElement | HTMLTextAreaElement, text: string) {
  const start = field.selectionStart ?? field.value.length;
  const end = field.selectionEnd ?? field.value.length;
  const next = field.value.slice(0, start) + text + field.value.slice(end);
  const prototype =
    field instanceof HTMLTextAreaElement
      ? HTMLTextAreaElement.prototype
      : HTMLInputElement.prototype;
  const setter = Object.getOwnPropertyDescriptor(prototype, "value")?.set;
  if (setter) setter.call(field, next);
  else field.value = next;
  const caret = start + text.length;
  field.setSelectionRange(caret, caret);
  field.dispatchEvent(new Event("input", { bubbles: true }));
}

/**
 * App-wide safety net for pastes that carry no text.
 *
 * The Linux selection can be empty while the clipboard genuinely holds text —
 * under WSL, WSLg does not always bridge the Windows selection across. Without
 * this, every plain input in the app silently inserts nothing.
 *
 * Deliberately on the BUBBLE phase and skipping `defaultPrevented`: anything that
 * already handles its own paste (the chat composer and the task description, which
 * also probe for images/files; Monaco, which inserts through the editor API) runs
 * first and opts out just by calling preventDefault.
 */
export function installPasteFallback(): () => void {
  const onPaste = (event: ClipboardEvent) => {
    if (event.defaultPrevented) return;
    if (event.clipboardData?.getData("text/plain")) return;
    const target = event.target as HTMLElement | null;
    const field = target?.closest?.("input, textarea") as
      | HTMLInputElement
      | HTMLTextAreaElement
      | null;
    if (!field || field.readOnly || field.disabled) return;
    if (field instanceof HTMLInputElement && !TEXT_INPUT_TYPES.has(field.type)) return;
    // Must be synchronous — the async read below cannot cancel the default action.
    event.preventDefault();
    void readClipboardText().then((text) => {
      if (text) insertIntoField(field, text);
    });
  };
  window.addEventListener("paste", onPaste);
  return () => window.removeEventListener("paste", onPaste);
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
