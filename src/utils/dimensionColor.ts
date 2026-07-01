// Dimension auto-coloring: spreads active dimensions evenly across the hue
// wheel, sorted by key so drag-reorder doesn't change colors. Deleted
// dimensions get a fixed neutral gray. All output is hsl() strings for use
// in inline :style — Tailwind can't scan dynamically-computed class names.

import type { Dimension } from "../types";

const BASE = 210; // starting hue — a pleasant blue for the single-dimension case

// ---- hue assignment ----------------------------------------------------

/** Map dimension key → hue (degrees, 0–360), or null for deleted dimensions.
 *  Active dimensions are sorted by key and spread evenly across the wheel. */
export function dimensionHues(
  dimensions: Dimension[],
): Map<string, number | null> {
  const active = dimensions
    .filter((d) => !d.deleted)
    .map((d) => d.key)
    .sort();
  const n = active.length;
  const map = new Map<string, number | null>();

  for (const d of dimensions) {
    if (d.deleted) {
      map.set(d.key, null);
    }
  }
  for (let i = 0; i < n; i++) {
    const hue = n === 1 ? BASE : (BASE + (i * 360) / n) % 360;
    map.set(active[i], Math.round(hue));
  }

  return map;
}

// ---- color producers (5 roles) -----------------------------------------

function hsl(h: number, s: number, l: number): string {
  return `hsl(${h} ${s}% ${l}%)`;
}

/** Bar color (3px left indicator).  hue=null → gray. */
export function dimBar(hue: number | null): string {
  return hue === null ? hsl(0, 0, 75) : hsl(hue, 58, 70);
}

/** Display-chip style (EntryRow, passive state). */
export function dimChipStyle(hue: number | null): {
  background: string;
  color: string;
} {
  return hue === null
    ? { background: hsl(0, 0, 96), color: hsl(0, 0, 45) }
    : { background: hsl(hue, 42, 96), color: hsl(hue, 40, 42) };
}

/** Token-chip style (EntryRowEdit, active editing state). */
export function dimTokenChipStyle(hue: number | null): {
  background: string;
  color: string;
} {
  return hue === null
    ? { background: hsl(0, 0, 95), color: hsl(0, 0, 40) }
    : { background: hsl(hue, 66, 95), color: hsl(hue, 60, 37) };
}
