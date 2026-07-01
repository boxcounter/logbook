import { describe, it, expect } from "vitest";
import {
  dimensionHues,
  dimBar,
  dimChipStyle,
  dimTokenChipStyle,
} from "../utils/dimensionColor";
import type { Dimension } from "../types";

function dim(overrides: Partial<Dimension> = {}): Dimension {
  return {
    name: overrides.key ?? "Test",
    key: overrides.key ?? "test",
    source: "static",
    required: false,
    deleted: false,
    values: ["a"],
    ...overrides,
  };
}

describe("dimensionHues", () => {
  it("returns empty map for empty input", () => {
    expect(dimensionHues([]).size).toBe(0);
  });

  it("single active dimension gets BASE hue", () => {
    const hues = dimensionHues([dim({ key: "goal" })]);
    expect(hues.get("goal")).toBe(210);
  });

  it("two active dimensions are 180° apart", () => {
    const hues = dimensionHues([dim({ key: "a" }), dim({ key: "b" })]);
    expect(hues.get("a")).toBe(210);
    expect(hues.get("b")).toBe(30); // (210 + 180) % 360
  });

  it("three active dimensions are 120° apart", () => {
    const hues = dimensionHues([
      dim({ key: "alpha" }),
      dim({ key: "beta" }),
      dim({ key: "gamma" }),
    ]);
    // Sorted by key: alpha, beta, gamma
    expect(hues.get("alpha")).toBe(210);
    expect(hues.get("beta")).toBe(330); // (210 + 120) % 360
    expect(hues.get("gamma")).toBe(90); // (210 + 240) % 360
  });

  it("sorts by key, not input order", () => {
    // Input in reverse key order — output should be key-sorted
    const hues = dimensionHues([
      dim({ key: "c" }),
      dim({ key: "a" }),
      dim({ key: "b" }),
    ]);
    expect(hues.get("a")).toBe(210);
    expect(hues.get("b")).toBe(330);
    expect(hues.get("c")).toBe(90);
  });

  it("adding a dimension changes hues (re-spread)", () => {
    const two = dimensionHues([dim({ key: "a" }), dim({ key: "b" })]);
    const three = dimensionHues([
      dim({ key: "a" }),
      dim({ key: "b" }),
      dim({ key: "c" }),
    ]);
    // With 2 dims: a→210, b→30. With 3: a→210, b→330, c→90.
    // b changes from 30° to 330°.
    expect(two.get("b")).not.toBe(three.get("b"));
  });

  it("deleted dimensions get null hue", () => {
    const hues = dimensionHues([
      dim({ key: "a" }),
      dim({ key: "z", deleted: true }),
    ]);
    expect(hues.get("a")).toBe(210); // only active dim
    expect(hues.get("z")).toBeNull();
  });

  it("deleted dimensions do not affect active hue count", () => {
    // 1 active + 2 deleted → still treated as N=1 (BASE)
    const hues = dimensionHues([
      dim({ key: "active" }),
      dim({ key: "del1", deleted: true }),
      dim({ key: "del2", deleted: true }),
    ]);
    expect(hues.get("active")).toBe(210);
  });
});

describe("dimBar", () => {
  it("produces an hsl string", () => {
    const bar = dimBar(210);
    expect(bar).toMatch(/^hsl\(\d+ 58% 70%\)$/);
  });

  it("uses gray for null hue", () => {
    expect(dimBar(null)).toBe("hsl(0 0% 75%)");
  });
});

describe("dimChipStyle", () => {
  it("returns background and color for an active hue", () => {
    const style = dimChipStyle(210);
    expect(style.background).toMatch(/^hsl\(\d+ 42% 96%\)$/);
    expect(style.color).toMatch(/^hsl\(\d+ 40% 42%\)$/);
  });

  it("returns gray for null hue", () => {
    const style = dimChipStyle(null);
    expect(style.background).toBe("hsl(0 0% 96%)");
    expect(style.color).toBe("hsl(0 0% 45%)");
  });
});

describe("dimTokenChipStyle", () => {
  it("returns background and color for an active hue", () => {
    const style = dimTokenChipStyle(210);
    expect(style.background).toMatch(/^hsl\(\d+ 66% 95%\)$/);
    expect(style.color).toMatch(/^hsl\(\d+ 60% 37%\)$/);
  });

  it("returns gray for null hue", () => {
    const style = dimTokenChipStyle(null);
    expect(style.background).toBe("hsl(0 0% 95%)");
    expect(style.color).toBe("hsl(0 0% 40%)");
  });
});
