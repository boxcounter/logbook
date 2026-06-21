import { describe, it, expect } from "vitest";

const vueFiles = import.meta.glob("../components/**/*.vue", {
  query: "?raw", import: "default", eager: true,
}) as Record<string, string>;
const appFile = import.meta.glob("../App.vue", {
  query: "?raw", import: "default", eager: true,
}) as Record<string, string>;
const allFiles = { ...vueFiles, ...appFile };

// --- Allowlist: files not yet migrated. Remove entries in Phase 3 as each file
// is migrated. The suite stays green; the goal is an EMPTY allowlist. ---
const ALLOWLIST = new Set<string>([
  "../App.vue",
  "../components/ConfigErrorBanner.vue",
  "../components/base/AppButton.vue",
  "../components/base/Toast.vue",
]);

const SPACE_PREFIXES =
  "p|px|py|pt|pb|pl|pr|m|mx|my|mt|mb|ml|mr|gap|gap-x|gap-y|space-x|space-y";
// Named scale (documentation): 2xs|xs|sm|md|lg|xl|2xl

// Map a px number to the suggested spacing suffix (canonical table in the plan).
function spacingSuffix(px: number): string {
  if (px <= 2) return "2xs";
  if (px <= 5) return "xs";
  if (px <= 10) return "sm";
  if (px <= 14) return "md";
  if (px <= 16) return "lg";
  if (px <= 24) return "xl";
  return "2xl";
}

function spacingViolations(src: string): string[] {
  const out: string[] = [];
  // arbitrary px: e.g. gap-[8px], px-[14px]
  const arb = new RegExp(`\\b(${SPACE_PREFIXES})-\\[(\\d+(?:\\.\\d+)?)px\\]`, "g");
  for (const m of src.matchAll(arb)) {
    const px = Math.round(Number(m[2]));
    if (px === 0) continue;
    out.push(`${m[0]} → ${m[1]}-${spacingSuffix(px)} (--spacing-${spacingSuffix(px)}); spacing must use the named scale`);
  }
  // numeric defaults: e.g. p-8, mx-4 (but NOT named like p-sm).
  const num = new RegExp(`\\b(${SPACE_PREFIXES})-(\\d+)\\b`, "g");
  for (const m of src.matchAll(num)) {
    const px = Number(m[2]) * 4;
    if (px === 0) continue;
    out.push(`${m[0]} → ${m[1]}-${spacingSuffix(px)} (=${px}px); use the named scale, not Tailwind's numeric default`);
  }
  return out;
}

function fontViolations(src: string): string[] {
  const out: string[] = [];
  // default Tailwind font-size utilities (text-xs … text-9xl) used standalone
  const def = /\btext-(xs|sm|base|lg|xl|[2-9]xl)\b/g;
  for (const m of src.matchAll(def)) {
    out.push(`${m[0]} → use a semantic size (text-title/body/secondary/micro)`);
  }
  // arbitrary font size: text-[length:...] or text-[16px]; allow ONLY the glyph var
  const arb = /\btext-\[(length:var\(--glyph-plus\)|\d+px|length:var\(--app-text-[a-z0-9-]+\))\]/g;
  for (const m of src.matchAll(arb)) {
    if (m[1] === "length:var(--glyph-plus)") continue; // sanctioned one-off
    out.push(`${m[0]} → use a semantic size (text-title/body/secondary/micro); see mapping table`);
  }
  return out;
}

describe("Tailwind token usage", () => {
  // Preserved original guard: font-size tokens must carry the length: hint.
  it("never references --app-text-* via text-[var(...)] without a length: hint", () => {
    const offenders: string[] = [];
    for (const [path, src] of Object.entries(allFiles)) {
      const matches = src.match(/text-\[var\(--app-text-[^\]]+\)\]/g);
      if (matches) offenders.push(`${path}: ${matches.join(", ")}`);
    }
    expect(offenders).toEqual([]);
  });

  it("uses only the named spacing scale (allowlist shrinking to empty)", () => {
    const offenders: string[] = [];
    for (const [path, src] of Object.entries(allFiles)) {
      if (ALLOWLIST.has(path)) continue;
      const v = spacingViolations(src);
      if (v.length) offenders.push(`${path}:\n  ${v.join("\n  ")}`);
    }
    expect(offenders).toEqual([]);
  });

  it("uses only the semantic font sizes (allowlist shrinking to empty)", () => {
    const offenders: string[] = [];
    for (const [path, src] of Object.entries(allFiles)) {
      if (ALLOWLIST.has(path)) continue;
      const v = fontViolations(src);
      if (v.length) offenders.push(`${path}:\n  ${v.join("\n  ")}`);
    }
    expect(offenders).toEqual([]);
  });

  it("has no stale allowlist entries (migrated files must be removed)", () => {
    const stale: string[] = [];
    for (const path of ALLOWLIST) {
      const src = allFiles[path];
      if (src && spacingViolations(src).length === 0 && fontViolations(src).length === 0) {
        stale.push(path);
      }
    }
    expect(stale).toEqual([]);
  });
});
