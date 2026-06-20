import { describe, it, expect } from "vitest";

/**
 * Regression guard for a Tauri-specific drag-and-drop bug.
 *
 * Tauri's webview enables an OS drag-drop handler by default, which intercepts
 * the native HTML5 `dragover`/`drop` events that Sortable.js relies on. The
 * result: drag starts (item floats) but never reorders — Sortable reports
 * newIndex === oldIndex because its dragover sorting never fires.
 *
 * The fix is `forceFallback: true`, which makes Sortable use pointer/mouse
 * events instead of native HTML5 DnD (so Tauri's handler doesn't intercept it).
 * `fallbackOnBody: true` keeps the drag clone from being clipped by the modal's
 * teleport + overflow-hidden.
 *
 * This behavior can't be verified in jsdom (no real DnD/layout), so this
 * source-level guard ensures the props aren't silently removed.
 */
const vueFiles = import.meta.glob("../components/composite/{CommitmentsModal,RoleCard}.vue", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

describe("Tauri DnD fallback guard", () => {
  it("every VueDraggable keeps forceFallback + fallbackOnBody (Tauri intercepts native DnD)", () => {
    const paths = Object.keys(vueFiles);
    expect(paths.length).toBe(2); // CommitmentsModal.vue + RoleCard.vue

    for (const [path, src] of Object.entries(vueFiles)) {
      const draggableLines = src.split("\n").filter(l => l.includes("<VueDraggable"));
      expect(draggableLines.length, `${path} should contain a <VueDraggable>`).toBeGreaterThan(0);
      for (const line of draggableLines) {
        expect(line, `${path}: VueDraggable missing :force-fallback`).toContain(':force-fallback="true"');
        expect(line, `${path}: VueDraggable missing :fallback-on-body`).toContain(':fallback-on-body="true"');
      }
    }
  });
});
