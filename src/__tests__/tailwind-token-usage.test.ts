import { describe, it, expect } from "vitest";

/**
 * Regression guard for a Tailwind v4 type-ambiguity bug.
 *
 * `text-[…]` serves both font-size and text-color. For an opaque CSS variable
 * Tailwind cannot infer the type and defaults to COLOR, so
 *   text-[var(--app-text-base)]   ->  color: var(--app-text-base)   (= color:14px, invalid, ignored)
 * Every font-size token silently fails and text falls back to the 16px default.
 *
 * Font-size tokens MUST carry the length hint:
 *   text-[length:var(--app-text-base)]  ->  font-size: var(--app-text-base)
 *
 * jsdom doesn't compute styles and the build doesn't validate arbitrary values,
 * so only this source-level guard catches a reintroduction.
 */
const vueFiles = import.meta.glob("../components/**/*.vue", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

describe("Tailwind font-size token usage", () => {
  it("never references --app-text-* via text-[var(...)] without a length: hint", () => {
    const offenders: string[] = [];
    for (const [path, src] of Object.entries(vueFiles)) {
      const matches = src.match(/text-\[var\(--app-text-[^\]]+\)\]/g);
      if (matches) offenders.push(`${path}: ${matches.join(", ")}`);
    }
    expect(offenders).toEqual([]);
  });
});
