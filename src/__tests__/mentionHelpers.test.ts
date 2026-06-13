import { describe, it, expect } from "vitest";
import { dimBarColor, getValueCount, firstUnfilledRequiredIndex } from "../utils/mentionHelpers";

// ============================================================
// dimBarColor
// ============================================================
describe("dimBarColor", () => {
  it('returns bg-blue-500 for "goal"', () => {
    expect(dimBarColor("goal")).toBe("bg-blue-500");
  });

  it('returns bg-amber-500 for "business-line"', () => {
    expect(dimBarColor("business-line")).toBe("bg-amber-500");
  });

  it('returns bg-pink-500 for "importance-urgency"', () => {
    expect(dimBarColor("importance-urgency")).toBe("bg-pink-500");
  });

  it('returns bg-green-500 for "category"', () => {
    expect(dimBarColor("category")).toBe("bg-green-500");
  });

  it("returns bg-gray-400 for an unknown key", () => {
    expect(dimBarColor("nonexistent")).toBe("bg-gray-400");
  });

  it("returns bg-gray-400 for an empty string", () => {
    expect(dimBarColor("")).toBe("bg-gray-400");
  });
});

// ============================================================
// getValueCount
// ============================================================
describe("getValueCount", () => {
  const monthlyGoalOptions = ["Goal A", "Goal B", "Goal C"];

  it("returns monthlyGoalOptions.length when source is 'monthly'", () => {
    const dims = [{ key: "goal", source: "monthly" }];
    expect(getValueCount(dims, "goal", monthlyGoalOptions)).toBe(3);
  });

  it("returns values.length for a static dimension with values", () => {
    const dims = [{ key: "category", source: "static", values: ["Cat1", "Cat2"] }];
    expect(getValueCount(dims, "category", monthlyGoalOptions)).toBe(2);
  });

  it("returns 0 for a static dimension with no values array", () => {
    const dims = [{ key: "category", source: "static" }];
    expect(getValueCount(dims, "category", monthlyGoalOptions)).toBe(0);
  });

  it("returns 0 for a static dimension with empty values array", () => {
    const dims = [{ key: "category", source: "static", values: [] }];
    expect(getValueCount(dims, "category", monthlyGoalOptions)).toBe(0);
  });

  it("returns 0 when the dimension key is not found", () => {
    const dims = [{ key: "goal", source: "static", values: ["G1"] }];
    expect(getValueCount(dims, "nonexistent", monthlyGoalOptions)).toBe(0);
  });

  it("returns 0 for empty dimensions array", () => {
    expect(getValueCount([], "goal", monthlyGoalOptions)).toBe(0);
  });

  it("returns monthlyGoalOptions.length (0) when monthly options list is empty", () => {
    const dims = [{ key: "goal", source: "monthly" }];
    expect(getValueCount(dims, "goal", [])).toBe(0);
  });
});

// ============================================================
// firstUnfilledRequiredIndex
// ============================================================
describe("firstUnfilledRequiredIndex", () => {
  it("returns the index of the first required item whose key is missing from dimValues", () => {
    const items = [
      { key: "goal", required: true },
      { key: "category", required: true },
    ];
    const dimValues: Record<string, string> = {};
    expect(firstUnfilledRequiredIndex(items, dimValues)).toBe(0);
  });

  it("returns the index of the second required item when the first is already filled", () => {
    const items = [
      { key: "goal", required: true },
      { key: "category", required: true },
    ];
    const dimValues = { goal: "some-value" };
    expect(firstUnfilledRequiredIndex(items, dimValues)).toBe(1);
  });

  it("returns 0 when all required items are filled", () => {
    const items = [
      { key: "goal", required: true },
      { key: "category", required: true },
    ];
    const dimValues = { goal: "G1", category: "C1" };
    expect(firstUnfilledRequiredIndex(items, dimValues)).toBe(0);
  });

  it("treats an empty string value as unfilled", () => {
    const items = [
      { key: "goal", required: true },
      { key: "category", required: true },
    ];
    const dimValues = { goal: "", category: "C1" };
    expect(firstUnfilledRequiredIndex(items, dimValues)).toBe(0);
  });

  it("skips non-required items when searching for unfilled required", () => {
    const items = [
      { key: "goal", required: false },
      { key: "category", required: true },
    ];
    const dimValues: Record<string, string> = {};
    expect(firstUnfilledRequiredIndex(items, dimValues)).toBe(1);
  });

  it("returns 0 when there are no required items", () => {
    const items = [
      { key: "goal", required: false },
      { key: "category" },
    ];
    const dimValues: Record<string, string> = {};
    expect(firstUnfilledRequiredIndex(items, dimValues)).toBe(0);
  });

  it("returns 0 for an empty items array", () => {
    expect(firstUnfilledRequiredIndex([], {})).toBe(0);
  });

  it("returns 0 when a required item has no key at all", () => {
    const items = [
      { required: true },
    ];
    const dimValues: Record<string, string> = {};
    // A required item with no key is effectively always "unfilled",
    // but since key is undefined, it's treated as missing and returns its index.
    expect(firstUnfilledRequiredIndex(items, dimValues)).toBe(0);
  });

  it("handles missing required fields (required: undefined) by treating them as not required", () => {
    const items = [
      { key: "goal" },
      { key: "category", required: true },
    ];
    const dimValues: Record<string, string> = {};
    expect(firstUnfilledRequiredIndex(items, dimValues)).toBe(1);
  });

  it("returns the index of the first unfilled even when later items are also unfilled", () => {
    const items = [
      { key: "goal", required: true },
      { key: "business-line", required: true },
      { key: "importance-urgency", required: true },
    ];
    const dimValues = { goal: "G1" };
    // business-line at index 1 is the first unfilled required
    expect(firstUnfilledRequiredIndex(items, dimValues)).toBe(1);
  });
});
