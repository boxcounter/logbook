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
const ALLOWLIST = new Set<string>([]);
const LEADING_ALLOWLIST = new Set<string>([
  "../components/MonthView.vue",
  "../components/composite/CommitmentsModal.vue",
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
  // px keyword utility (e.g. gap-px = 1px)
  const pxKw = new RegExp(`\\b(${SPACE_PREFIXES})-px\\b`, "g");
  for (const m of src.matchAll(pxKw)) {
    out.push(`${m[0]} → ${m[1]}-2xs (--spacing-2xs); use the named scale`);
  }
  // arbitrary spacing in a non-px unit (the px form is handled above)
  const arbOther = new RegExp(`\\b(${SPACE_PREFIXES})-\\[(?![0-9.]+px\\])[^\\]]+\\]`, "g");
  for (const m of src.matchAll(arbOther)) {
    out.push(`${m[0]} → use the named --spacing-* scale (gap-sm/p-md/…); arbitrary spacing is not allowed`);
  }
  return out;
}

function leadingViolations(src: string): string[] {
  const out: string[] = [];
  // 任意行高: leading-[1.4] / leading-[1.8] / leading-[2rem] …（leading-none 不在此列）
  const arb = /\bleading-\[[^\]]+\]/g;
  for (const m of src.matchAll(arb)) {
    out.push(`${m[0]} → 行高跟随字号档；需紧排用 leading-none（破例需注释 + 显式豁免）`);
  }
  // Tailwind 数字档: leading-6 / leading-7
  const num = /\bleading-\d+\b/g;
  for (const m of src.matchAll(num)) {
    out.push(`${m[0]} → 行高跟随字号档；需紧排用 leading-none`);
  }
  // 具名非 none 档: leading-tight/snug/normal/relaxed/loose
  const named = /\bleading-(tight|snug|normal|relaxed|loose)\b/g;
  for (const m of src.matchAll(named)) {
    out.push(`${m[0]} → 行高跟随字号档；需紧排用 leading-none`);
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

  it("uses only the sanctioned leading utility (leading-none); shrinking to empty", () => {
    const offenders: string[] = [];
    for (const [path, src] of Object.entries(allFiles)) {
      if (LEADING_ALLOWLIST.has(path)) continue;
      const v = leadingViolations(src);
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
    for (const path of LEADING_ALLOWLIST) {
      const src = allFiles[path];
      if (src && leadingViolations(src).length === 0) {
        stale.push(`${path} (leading)`);
      }
    }
    expect(stale).toEqual([]);
  });
});
