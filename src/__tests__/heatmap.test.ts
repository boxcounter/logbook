// src/__tests__/heatmap.test.ts
import { describe, it, expect } from "vitest";
import { heatLevel } from "../utils/heatmap";

describe("heatLevel", () => {
  it("returns 'empty' for zero or negative minutes", () => {
    expect(heatLevel(0)).toBe("empty");
    expect(heatLevel(-5)).toBe("empty");
  });
  it("returns 'light' for under 2h", () => {
    expect(heatLevel(1)).toBe("light");
    expect(heatLevel(119)).toBe("light");
  });
  it("returns 'mid' for 2h to under 5h", () => {
    expect(heatLevel(120)).toBe("mid");
    expect(heatLevel(299)).toBe("mid");
  });
  it("returns 'heavy' for 5h and above", () => {
    expect(heatLevel(300)).toBe("heavy");
    expect(heatLevel(600)).toBe("heavy");
  });
});
