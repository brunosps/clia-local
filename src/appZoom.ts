import { getCurrentWebview } from "@tauri-apps/api/webview";

/**
 * App zoom, the same idea as a browser's Ctrl+/Ctrl-: everything scales together.
 *
 * Uses the webview's own zoom rather than a CSS `zoom`/`transform` on the root.
 * Monaco reads layout geometry straight from the DOM to place the cursor and the
 * hit-test targets; a CSS scale leaves those coordinates in unscaled space and
 * clicks land on the wrong character. Native zoom scales the whole viewport, so
 * the editor stays consistent.
 */

export const APP_ZOOM_KEY = "ui.app_zoom";
export const DEFAULT_APP_ZOOM = 100;
export const APP_ZOOM_STEPS = [80, 90, 100, 110, 125, 150, 175, 200] as const;

const MIN_ZOOM = APP_ZOOM_STEPS[0];
const MAX_ZOOM = APP_ZOOM_STEPS[APP_ZOOM_STEPS.length - 1];

/**
 * Clamp an arbitrary stored value to a usable percentage.
 *
 * Absent values are checked before the numeric coercion on purpose: `Number(null)`
 * and `Number("")` are `0`, which is finite, so an unset preference would clamp to
 * the minimum zoom instead of falling back to 100%.
 */
export function normalizeAppZoom(value: unknown): number {
  if (value == null || value === "") return DEFAULT_APP_ZOOM;
  const parsed = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(parsed)) return DEFAULT_APP_ZOOM;
  return Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, Math.round(parsed)));
}

/** The next/previous step, for the Ctrl+= / Ctrl+- shortcuts. */
export function stepAppZoom(current: number, direction: 1 | -1): number {
  const zoom = normalizeAppZoom(current);
  const steps = [...APP_ZOOM_STEPS];
  if (direction === 1) return steps.find((step) => step > zoom) ?? zoom;
  return [...steps].reverse().find((step) => step < zoom) ?? zoom;
}

/**
 * Apply a zoom percentage to the webview. Returns false when the platform does
 * not support it (the caller can then leave the stored preference alone rather
 * than pretend it took effect).
 */
export async function applyAppZoom(percent: number): Promise<boolean> {
  try {
    await getCurrentWebview().setZoom(normalizeAppZoom(percent) / 100);
    return true;
  } catch {
    return false;
  }
}
