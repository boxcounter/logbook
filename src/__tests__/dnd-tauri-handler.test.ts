import { describe, it, expect } from "vitest";

/**
 * Regression guard for a Tauri-specific drag-and-drop bug.
 *
 * Tauri's webview enables an OS drag-drop handler by default, which intercepts
 * the native HTML5 `dragover`/`drop` events that Sortable.js relies on. The
 * result: in-app drag-reorder floated but never reordered — Sortable reported
 * newIndex === oldIndex because its dragover sorting never fired.
 *
 * Root-cause fix: disable that handler on the main window (we don't use OS
 * file-drop), so native HTML5 DnD reaches Sortable app-wide.
 *
 * Real DnD can't be exercised in jsdom, so this source-level guard ensures the
 * handler stays disabled — re-enabling it silently breaks every drag-reorder.
 */
const libFiles = import.meta.glob("../../src-tauri/src/lib.rs", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

describe("Tauri DnD handler guard", () => {
  it("disables Tauri's OS drag-drop handler so native HTML5 DnD reaches Sortable", () => {
    const lib = Object.values(libFiles)[0] ?? "";
    expect(lib, "src-tauri/src/lib.rs should be readable").not.toBe("");
    expect(lib).toContain(".disable_drag_drop_handler()");
  });
});
