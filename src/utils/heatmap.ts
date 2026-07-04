// src/utils/heatmap.ts
export type HeatLevel = "empty" | "light" | "mid" | "heavy";

const LIGHT_THRESHOLD_MINUTES = 120;
const MID_THRESHOLD_MINUTES = 300;

/** Classify a day's total logged minutes into a heatmap intensity bucket.
 *  Thresholds: 0 → empty, <2h → light, <5h → mid, >=5h → heavy. */
export function heatLevel(totalMinutes: number): HeatLevel {
  if (totalMinutes <= 0) return "empty";
  if (totalMinutes < LIGHT_THRESHOLD_MINUTES) return "light";
  if (totalMinutes < MID_THRESHOLD_MINUTES) return "mid";
  return "heavy";
}
