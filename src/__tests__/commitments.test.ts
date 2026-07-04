import { describe, it, expect } from "vitest";
import { goalLoggedMinutes } from "../utils/commitments";
import type { CommitmentProgress } from "../types";

const makeProgress = (
  role: string,
  goalSpentPairs: [string, number][],
): CommitmentProgress => ({
  role,
  allocation_minutes: 2400,
  goal_spent_minutes: goalSpentPairs.reduce((s, [, m]) => s + m, 0),
  general_spent_minutes: 0,
  goals: goalSpentPairs.map(([name, spent_minutes]) => ({ name, spent_minutes })),
});

describe("goalLoggedMinutes", () => {
  it("returns 0 for null name", () => {
    expect(goalLoggedMinutes([], null)).toBe(0);
  });

  it("returns 0 for empty progress array", () => {
    expect(goalLoggedMinutes([], "Bug fixes")).toBe(0);
  });

  it("returns 0 when goal not found", () => {
    const progress = [makeProgress("Dev", [["Feature A", 120]])];
    expect(goalLoggedMinutes(progress, "Bug fixes")).toBe(0);
  });

  it("returns spent minutes when goal found", () => {
    const progress = [makeProgress("Dev", [["Bug fixes", 90]])];
    expect(goalLoggedMinutes(progress, "Bug fixes")).toBe(90);
  });

  it("finds goal across multiple roles", () => {
    const progress = [
      makeProgress("Dev", [["Feature A", 60]]),
      makeProgress("VP", [["Strategy", 120]]),
    ];
    expect(goalLoggedMinutes(progress, "Strategy")).toBe(120);
  });
});
