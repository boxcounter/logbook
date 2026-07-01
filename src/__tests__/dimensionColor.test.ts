import { describe, it, expect } from "vitest";
import { dimBarVar, dimBarColor, dimBarClass } from "../utils/dimensionColor";

describe("dimensionColor", () => {
  // The bug: full dimension keys don't match the abbreviated CSS variable
  // suffixes. These map assertions lock the mapping so the modal and popover
  // can't diverge again.
  it("maps full keys to abbreviated CSS variable names", () => {
    expect(dimBarVar("category")).toBe("--dim-bar-cat");
    expect(dimBarVar("business-line")).toBe("--dim-bar-biz");
    expect(dimBarVar("importance-urgency")).toBe("--dim-bar-imp");
    expect(dimBarVar("goal")).toBe("--dim-bar-goal");
  });

  it("falls back to --dim-bar-cat for unknown keys", () => {
    expect(dimBarVar("some-custom-key")).toBe("--dim-bar-cat");
  });

  it("dimBarColor wraps the variable in var()", () => {
    expect(dimBarColor("importance-urgency")).toBe("var(--dim-bar-imp)");
    expect(dimBarColor("unknown")).toBe("var(--dim-bar-cat)");
  });

  it("dimBarClass produces a Tailwind bg utility", () => {
    expect(dimBarClass("business-line")).toBe("bg-[var(--dim-bar-biz)]");
    expect(dimBarClass("goal")).toBe("bg-[var(--dim-bar-goal)]");
  });
});
