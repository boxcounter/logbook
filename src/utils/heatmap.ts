// src/utils/heatmap.ts
export type HeatLevel = "empty" | "light" | "mid" | "heavy";

/** Classify a day's total logged minutes into a heatmap intensity bucket.
 *  Thresholds: 0 → empty, <2h → light, <5h → mid, >=5h → heavy. */
export function heatLevel(totalMinutes: number): HeatLevel {
  if (totalMinutes <= 0) return "empty";
  if (totalMinutes < 120) return "light";
  if (totalMinutes < 300) return "mid";
  return "heavy";
}
