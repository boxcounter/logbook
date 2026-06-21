# Design System Consolidation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Consolidate the front-end design tokens (type scale 6→4, spacing 16-literals→7-grid, motion naming, radius cleanup) into Tailwind v4 `@theme` scales, and add executable + in-context governance so the scales can't silently rot again.

**Architecture:** Define utility-generating scales (`--text-*`, `--spacing-*`) in a `@theme` block in `src/assets/main.css`; keep non-utility tokens (colors, radius, shadows, motion) as `:root` vars in `src/assets/tokens.css`. Migrate every component from arbitrary values (`text-[length:var(--app-text-base)]`, `gap-[8px]`) to named utilities (`text-body`, `gap-sm`) using a ratchet: an extended `tailwind-token-usage.test.ts` carries an allowlist of not-yet-migrated files and shrinks to empty. Wire the guard into vitest + a new pre-commit hook + CI.

**Tech Stack:** Vue 3 + TypeScript, Tailwind CSS v4 (`@tailwindcss/vite`), Vitest, vue-tsc, Tauri 2. New dev deps in Phase 4: husky, lint-staged.

**Spec:** `docs/superpowers/specs/2026-06-21-design-system-consolidation-design.md`

---

## Conventions used by every task

- **Project rule — Phase checkpoint:** stop at the end of each Phase and get confirmation before starting the next. Do not chain phases.
- **Build vs test:** `npm run build` = `vue-tsc --noEmit && vite build` — it **typechecks** tests strictly (noUnusedLocals over `*.test.ts`) but does **not run** them. The guard test runs only under `npm test` (`vitest run`). After any migration task, run **both** `npm test` and `npm run build`.
- **Visual check:** after a migration commit, the executor (or user) eyeballs the affected component in `npm run tauri dev` against the approved mockups in `.superpowers/brainstorm/` before moving on.
- **Commit cadence:** one commit per task unless stated otherwise.

---

## Canonical mapping tables (single source — referenced by every migration task)

### Type scale: old arbitrary class → new named utility

| Old usage in components | New utility |
|---|---|
| `text-[length:var(--app-text-xl)]` (20) | `text-title` |
| `text-[length:var(--app-text-base)]` (14) | `text-body` |
| `text-[length:var(--app-text-sm)]` (13) | `text-secondary` |
| `text-[length:var(--app-text-xs)]` (12) | `text-secondary` |
| `text-[length:var(--app-text-xs-alt)]` (11) | `text-secondary` |
| `text-[length:var(--app-text-micro)]` (10) | `text-micro` |
| `text-[length:var(--app-text-2xs)]` (9) | `text-micro` |
| `text-[length:var(--app-text-lg)]` (18, the `+` glyph) | `text-[length:var(--glyph-plus)]` (kept arbitrary — not a scale tier) |
| `text-sm` (raw default, `App.vue:155`) | `text-secondary` |
| `text-[16px]` (SetupScreen CTA) | **decision point** — map to `text-body`; if 14 reads too small for the CTA, keep `text-[16px]` with an inline `/* one-off CTA size, signed off */` comment per the escape-hatch rule (§3.1 of spec). Default: `text-body`. |

### Spacing scale: px value → named utility suffix

Applies to spacing props only: `p px py pt pb pl pr m mx my mt mb ml mr gap gap-x gap-y space-x space-y`.

| px (arbitrary or default-derived) | suffix | token |
|---|---|---|
| 1, 2 | `2xs` | `--spacing-2xs: 2px` |
| 3, 4, 5 | `xs` | `--spacing-xs: 4px` |
| 6, 7, 8, 9, 10 | `sm` | `--spacing-sm: 8px` |
| 12, 14 | `md` | `--spacing-md: 12px` |
| 16 | `lg` | `--spacing-lg: 16px` |
| 20, 24 | `xl` | `--spacing-xl: 24px` |
| 28 | `2xl` | `--spacing-2xl: 32px` |

Tailwind numeric defaults map by their px value (default `--spacing` = 0.25rem, so `p-N` = `N×4px`): `p-2`=8→`p-sm`, `p-4`=16→`p-lg`, `p-8`=32→`p-2xl`, `mx-4`/`mt-4`=16→`mx-lg`/`mt-lg`.

**Out of scope for spacing guard:** sizing utilities `w- h- min-w- min-h- max-w- max-h-` keep arbitrary px (component dimensions like `w-[42px]`, `h-[4px]`, `min-w-[46px]` are legitimately off-grid). The guard does NOT touch them.

### Motion: magic number → CSS var (soft, no guard)

| Old | New |
|---|---|
| `duration-150` | `duration-[var(--motion-fast)]` (150ms) |
| `duration-200` | `duration-[var(--motion-base)]` (200ms) |
| `duration-500` | `duration-[var(--motion-slow)]` (500ms) |
| `transition: all 0.2s ease-out` (Toast scoped CSS) | `transition: all var(--motion-base) var(--ease-out)` |
| `var(--anim-highlight-duration)` (1.5s) | unchanged |

### Radius: escapee → token

| Escapee | File:line | New |
|---|---|---|
| `rounded-[4px]` ×2 | `base/ProgressBar.vue:10,12` | `rounded-full` |
| `rounded-[2px]` ×2 | `composite/RoleCard.vue:152,155` | `rounded-full` |
| `rounded-[10px]` | `base/Toast.vue:22` | `rounded-[var(--radius-card)]` (12) |
| `rounded-lg` ×2 | `ConfigErrorBanner.vue:7`, `App.vue:155` | `rounded-[var(--radius-form-lg)]` (8) |

---

## Phase 0: Baseline

### Task 0.1: Confirm green baseline

**Files:** none (verification only)

- [ ] **Step 1: Run the full test suite**

Run: `npm test`
Expected: all suites PASS (includes `src/__tests__/tailwind-token-usage.test.ts`).

- [ ] **Step 2: Run the build**

Run: `npm run build`
Expected: PASS (vue-tsc clean, vite build succeeds).

- [ ] **Step 3: Record the current spacing/font literal inventory** (used to verify migration completeness later)

Run:
```bash
grep -rhoE "(p|px|py|pt|pb|pl|pr|m|mx|my|mt|mb|ml|mr|gap|gap-x|gap-y)-\[[0-9.]+px\]" src/components src/App.vue | sort | uniq -c
grep -rhoE "text-\[length:var\(--app-text-[a-z0-9-]+\)\]" src/components src/App.vue | sort | uniq -c
```
Expected: a non-empty inventory. Save the output to the task notes — Phase 3 drives this to zero.

**No commit** (read-only).

---

## Phase 1: Token foundation (scales exist; no migration yet)

End state: new utilities `text-title/body/secondary/micro`, `gap-sm` … `p-2xl` all compile and coexist with the old arbitrary values. Nothing migrated yet, so the app is visually unchanged.

### Task 1.1: Add the type scale to `@theme`

**Files:**
- Modify: `src/assets/main.css:1-2` (add `@theme` block after the tailwindcss import)

- [ ] **Step 1: Add the `@theme` type scale**

In `src/assets/main.css`, immediately after `@import 'tailwindcss';`, add:

```css
@theme {
  /* === Type scale (4 semantic tiers) ===
     Defining --text-* generates the text-* font-size utilities. We deliberately
     use the --text-* namespace (the old --app-text-* prefix existed only to AVOID
     it while tokens lived in :root). Line-heights match the prior per-element values. */
  --text-title: 20px;
  --text-title--line-height: 1.2;
  --text-body: 14px;
  --text-body--line-height: 1.4;
  --text-secondary: 12px;
  --text-secondary--line-height: 1.5;
  --text-micro: 10px;
  --text-micro--line-height: 1.6;
}
```

- [ ] **Step 2: Add a temporary probe component to verify `text-secondary` resolves to font-size (not color)**

Create `src/__tests__/theme-probe.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import { defineComponent, h } from "vue";

// `text-*` is overloaded (font-size AND color). Confirm our custom keys land as
// utilities at all (jsdom can't compute CSS, so we only assert the class is
// accepted and rendered — real font-size is confirmed by the visual check).
describe("theme type-scale utilities", () => {
  it("renders the four named text utilities without error", () => {
    const C = defineComponent({
      render: () => h("div", [
        h("span", { class: "text-title" }),
        h("span", { class: "text-body" }),
        h("span", { class: "text-secondary" }),
        h("span", { class: "text-micro" }),
      ]),
    });
    const w = mount(C);
    expect(w.findAll("span")).toHaveLength(4);
  });
});
```

- [ ] **Step 3: Verify build picks up the new utilities**

Run: `npm run build`
Expected: PASS. (vite build compiles the CSS; if `text-secondary` were rejected as ambiguous, you'd see no error here but a missing class at runtime — the Phase 3 visual check is the real confirmation. If during Phase 3 visual check `text-secondary` renders as a color, rename the token to `--text-meta` and update the mapping table; no color named `secondary` exists today so this is not expected.)

- [ ] **Step 4: Run tests**

Run: `npm test`
Expected: PASS (including the new probe).

- [ ] **Step 5: Commit**

```bash
git add src/assets/main.css src/__tests__/theme-probe.test.ts
git commit -m "feat(tokens): add 4-tier type scale to @theme (text-title/body/secondary/micro)"
```

### Task 1.2: Add the spacing scale to `@theme`

**Files:**
- Modify: `src/assets/main.css` (extend the `@theme` block)

- [ ] **Step 1: Add the spacing scale inside the existing `@theme` block**

```css
  /* === Spacing scale (7-tier, 4px grid + 2px micro) ===
     Defining named --spacing-* keys generates named utilities (gap-md, p-sm, mt-lg…)
     alongside Tailwind's numeric multiplier. The guard test forbids the numeric/
     arbitrary forms and allows only these names. */
  --spacing-2xs: 2px;
  --spacing-xs: 4px;
  --spacing-sm: 8px;
  --spacing-md: 12px;
  --spacing-lg: 16px;
  --spacing-xl: 24px;
  --spacing-2xl: 32px;
```

- [ ] **Step 2: Extend the probe test**

In `src/__tests__/theme-probe.test.ts`, add a second `it`:

```ts
  it("renders named spacing utilities without error", () => {
    const C = defineComponent({
      render: () => h("div", { class: "gap-sm p-md mt-lg px-2xs py-2xl" }),
    });
    const w = mount(C);
    expect(w.find("div").classes()).toContain("gap-sm");
  });
```

- [ ] **Step 3: Build + test**

Run: `npm run build && npm test`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/assets/main.css src/__tests__/theme-probe.test.ts
git commit -m "feat(tokens): add 7-tier spacing scale to @theme"
```

### Task 1.3: Add motion vars and the `+` glyph size to `:root`

**Files:**
- Modify: `src/assets/tokens.css` (Animation section + a new glyph entry)

- [ ] **Step 1: Add motion + glyph tokens**

In `src/assets/tokens.css`, in the `=== Animation ===` block, add (keep `--anim-highlight-*` as-is):

```css
  /* Motion durations (naming existing values; behavior unchanged) */
  --motion-fast: 150ms;   /* hover / color / border / opacity micro-feedback */
  --motion-base: 200ms;   /* buttons, toast enter/leave */
  --motion-slow: 500ms;   /* progress-bar fill (watchable) */
  --ease-out: cubic-bezier(0, 0, 0.2, 1);
  --ease-in: cubic-bezier(0.4, 0, 1, 1);
```

In the `=== Typography: Size ===` block, add after the size tokens:

```css
  --glyph-plus: 18px;     /* decorative "+" in the composer; NOT a text tier */
```

- [ ] **Step 2: Build + test**

Run: `npm run build && npm test`
Expected: PASS (no consumers yet; pure additions).

- [ ] **Step 3: Commit**

```bash
git add src/assets/tokens.css
git commit -m "feat(tokens): add --motion-*, --ease-*, and --glyph-plus tokens"
```

### ⛔ Phase 1 checkpoint — stop and confirm before Phase 2.

---

## Phase 2: Extend the guard test (ratchet) + human review

End state: `tailwind-token-usage.test.ts` enforces named spacing + named font sizes, with instructive failure messages, and an allowlist of the still-unmigrated files so the suite stays green. **Human reviews the guard's coverage before migration starts.**

### Task 2.1: Build the violation detectors with instructive messages

**Files:**
- Modify: `src/__tests__/tailwind-token-usage.test.ts`

- [ ] **Step 1: Replace the test file with the extended guard**

The existing single test (the `text-[var(--app-text-…)]` length-hint guard) is preserved as one of several checks. Full new file:

```ts
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
  // populated in Step 3 from the current offender list
]);

const SPACE_PREFIXES =
  "p|px|py|pt|pb|pl|pr|m|mx|my|mt|mb|ml|mr|gap|gap-x|gap-y|space-x|space-y";
const NAMED = "2xs|xs|sm|md|lg|xl|2xl";

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
    out.push(`${m[0]} → ${m[1]}-${spacingSuffix(px)} (--spacing-${spacingSuffix(px)}); spacing must use the named scale`);
  }
  // numeric defaults: e.g. p-8, mx-4 (but NOT named like p-sm). Exclude w/h handled
  // by prefix list (sizing prefixes are intentionally absent).
  const num = new RegExp(`\\b(${SPACE_PREFIXES})-(\\d+)\\b`, "g");
  for (const m of src.matchAll(num)) {
    const px = Number(m[2]) * 4;
    out.push(`${m[0]} → ${m[1]}-${spacingSuffix(px)} (=${px}px); use the named scale, not Tailwind's numeric default`);
  }
  return out;
}

function fontViolations(src: string): string[] {
  const out: string[] = [];
  const NAMED_TEXT = "title|body|secondary|micro";
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
```

- [ ] **Step 2: Run the guard to see the real offender list**

Temporarily set `ALLOWLIST` empty and run:
Run: `npm test -- tailwind-token-usage`
Expected: the two new tests FAIL, printing every offending file with `path → suggested` lines. Copy the set of offending file paths.

- [ ] **Step 3: Seed the allowlist with exactly those paths**

Paste the offending paths (the `../components/...` / `../App.vue` keys as they appear) into the `ALLOWLIST` set.

- [ ] **Step 4: Run again — now green**

Run: `npm test -- tailwind-token-usage`
Expected: PASS (all offenders allowlisted; the "stale allowlist" test passes because none are migrated yet).

- [ ] **Step 5: Full build + test**

Run: `npm run build && npm test`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/__tests__/tailwind-token-usage.test.ts
git commit -m "test(guard): enforce named spacing + semantic font sizes (ratchet allowlist)"
```

### ⛔ Phase 2 checkpoint — **HUMAN REVIEW REQUIRED.**
Per spec §3.1, a human (or one-off human pass) reviews the guard's coverage before migration: are the regexes too loose/tight? Are the instructive messages correct? Is the sizing-prefix exclusion right? Confirm before Phase 3.

---

## Phase 3: Migrate components (guard-driven ratchet)

For every file: remove it from `ALLOWLIST` (its test goes red, printing the exact `→` replacements), apply the mapping tables, run guard (green) + `npm run build` + `npm test`, visual-check, commit. Migrate font + spacing + radius + motion for a file together. Group below is by area to keep commits coherent; one commit per task.

> **Worked example (the pattern for all tasks).** In `composite/EntryRow.vue`: `px-[14px] py-[9px]` → `px-md py-sm`; `gap-[8px]` → `gap-sm`; `gap-[3px] mt-[3px]` → `gap-xs mt-xs`; `px-[6px]` (chip) → `px-sm`; `ml-[16px]` → `ml-lg`; `ml-[8px] px-[2px]` → `ml-sm px-2xs`; `text-[length:var(--app-text-base)]` → `text-body`; `text-[length:var(--app-text-micro)]` (chip) → `text-micro`; `text-[length:var(--app-text-sm)]` (duration) → `text-secondary`; `text-[14px]` (⋯ trigger) → `text-body`.

### Task 3.1: Day view — `DayHeader.vue`, `EntryList.vue`, `composite/EntryRow.vue`, `composite/EntryRowEdit.vue`

**Files:** Modify each; Test: `src/__tests__/tailwind-token-usage.test.ts` (allowlist edit)

- [ ] **Step 1:** Remove these four paths from `ALLOWLIST`.
- [ ] **Step 2:** Run `npm test -- tailwind-token-usage` → FAIL, listing each file's `→` replacements.
- [ ] **Step 3:** Apply every listed replacement per the mapping tables (font + spacing). `EntryRowEdit.vue` is not shown in this plan's reads — rely on the guard's printed `→` lines + the canonical tables; do not invent values.
- [ ] **Step 4:** Run `npm test -- tailwind-token-usage` → PASS for these files (others still allowlisted).
- [ ] **Step 5:** Run `npm run build && npm test` → PASS.
- [ ] **Step 6:** Visual check day view (header title 20, entry rows, chips, durations) against `component-preview.html` / `spacing-fullpage.html`.
- [ ] **Step 7:** Commit: `git commit -am "refactor(day-view): migrate to named type + spacing scales"`

### Task 3.2: Composer — `TwoLineInput.vue`, `DimensionPopover.vue`, `QuickJumpPopover.vue`

**Files:** Modify each; Test: allowlist edit

- [ ] **Step 1:** Remove the three paths from `ALLOWLIST`.
- [ ] **Step 2:** Run guard → FAIL with `→` lines.
- [ ] **Step 3:** Apply replacements. Note `TwoLineInput.vue`: the `+` glyph `text-[length:var(--app-text-lg)]` → `text-[length:var(--glyph-plus)]` (NOT a text tier); `⏎` badge and kbd `text-[length:var(--app-text-2xs)]` → `text-micro`; tokens `text-[length:var(--app-text-micro)]` → `text-micro`; `px-[16px] py-[10px]` → `px-lg py-sm`; `gap-[8px]`→`gap-sm`; `mt-[6px]`→`mt-sm`; `gap-[4px]`→`gap-xs`; `px-[7px] py-[1px]`→`px-sm py-2xs`; `px-[8px]`→`px-sm`; `gap-[14px] mt-[4px]`→`gap-md mt-xs`; `px-[5px]`→`px-xs`; `w-[5px] h-[5px]` and `min-h-[4px]` are **sizing — leave arbitrary**.
- [ ] **Step 4–7:** guard PASS → `npm run build && npm test` PASS → visual check composer + popovers (against `spacing-fullpage.html`) → commit: `git commit -am "refactor(composer): migrate to named type + spacing scales"`

### Task 3.3: Commitments — `composite/RoleCard.vue`, `composite/GoalRow.vue`, `composite/CommitmentsModal.vue`, `CommitmentsPanel.vue`

**Files:** Modify each; Test: allowlist edit

- [ ] **Step 1:** Remove the four paths from `ALLOWLIST`.
- [ ] **Step 2:** Run guard → FAIL with `→` lines.
- [ ] **Step 3:** Apply replacements. Includes the radius fix in `RoleCard.vue:152,155`: `rounded-[2px]` → `rounded-full` (progress-bar track + fill). Role name `text-[length:var(--app-text-base)]`→`text-body`; goal name `text-[length:var(--app-text-sm)]`→`text-secondary`; `text-[length:var(--app-text-xs-alt)]` (h unit, spent, delete)→`text-secondary`; `text-[length:var(--app-text-xs)]`→`text-secondary`. `w-[24px] h-[26px] w-[42px] min-w-[46px]` are **sizing — leave**.
- [ ] **Step 4–7:** guard PASS → `npm run build && npm test` PASS → visual check commitments (against `role-goal-distinction.html`: Role 14 / Goal 12 stay distinct) → commit: `git commit -am "refactor(commitments): migrate scales + unify progress-bar radius"`

### Task 3.4: Calendar/setup — `HeatmapCalendar.vue`, `MonthView.vue`, `SetupScreen.vue`

**Files:** Modify each; Test: allowlist edit

- [ ] **Step 1:** Remove the three paths from `ALLOWLIST`.
- [ ] **Step 2:** Run guard → FAIL with `→` lines.
- [ ] **Step 3:** Apply replacements. `SetupScreen.vue:67` CTA: `text-[16px]` → `text-body` (the decision-point default); `px-[24px] py-[12px]` → `px-xl py-md`. If 14px reads too small for the CTA during visual check, revert that one to `text-[16px]` and add `/* one-off CTA size, signed off */` + keep it off the allowlist by adding `text-[16px]` to the font guard's sanctioned set (mirror the `--glyph-plus` exception).
- [ ] **Step 4–7:** guard PASS → `npm run build && npm test` PASS → visual check heatmap/month/setup → commit: `git commit -am "refactor(calendar+setup): migrate to named scales"`

### Task 3.5: Base + edge UI — `base/ProgressBar.vue`, `base/Toast.vue`, `base/AppButton.vue`, `ConfigErrorBanner.vue`, `App.vue`

**Files:** Modify each; Test: allowlist edit

- [ ] **Step 1:** Remove the five paths from `ALLOWLIST`.
- [ ] **Step 2:** Run guard → FAIL with `→` lines.
- [ ] **Step 3:** Apply replacements, including:
  - `ProgressBar.vue:10,12`: `rounded-[4px]` → `rounded-full`.
  - `Toast.vue:22`: `rounded-[10px]` → `rounded-[var(--radius-card)]`; scoped CSS `transition: all 0.2s ease-out/ease-in` → `var(--motion-base) var(--ease-out)` / `var(--ease-in)`.
  - `ConfigErrorBanner.vue:7`: `rounded-lg`→`rounded-[var(--radius-form-lg)]`; raw `p-4 mx-4 mt-4`→`p-lg mx-lg mt-lg`.
  - `App.vue:155`: `rounded-lg`→`rounded-[var(--radius-form-lg)]`; `mx-4 mt-4 px-4 py-2`→`mx-lg mt-lg px-lg py-sm`; `text-sm`→`text-secondary` (this is a dev/error button; keep `bg-blue-600` as-is — color is out of scope).
- [ ] **Step 4:** Migrate motion durations across all already-migrated files too (soft, no guard): replace `duration-150/200/500` with `duration-[var(--motion-fast/base/slow)]` in `EntryRow`, `RoleCard`, `AppButton`, `SetupScreen`, `Toast`, etc. Grep `grep -rn "duration-150\|duration-200\|duration-500" src` to find all.
- [ ] **Step 5:** Run guard → PASS; the **"no stale allowlist entries"** test now confirms `ALLOWLIST` is empty (remove any leftover entries).
- [ ] **Step 6:** Run `npm run build && npm test` → PASS.
- [ ] **Step 7:** Visual check toasts, buttons, progress bars, error banner; confirm Phase 0 inventory greps now return empty for spacing/font literals.
- [ ] **Step 8:** Commit: `git commit -am "refactor(base+edge): migrate scales, radius escapees, motion vars"`

### ⛔ Phase 3 checkpoint — stop and confirm. Whole app migrated; allowlist empty.

---

## Phase 4: Wire the gate (in-session verify + pre-commit + CI)

### Task 4.1: Add a combined `verify` script

**Files:** Modify `package.json`

- [ ] **Step 1:** Add to `scripts`:

```json
    "verify": "vitest run && vue-tsc --noEmit && vite build"
```

- [ ] **Step 2:** Run `npm run verify` → PASS.
- [ ] **Step 3:** Commit: `git commit -am "chore: add npm run verify (test + typecheck + build gate)"`

### Task 4.2: Pre-commit hook via husky + lint-staged

**Files:** Create `.husky/pre-commit`; Modify `package.json`

- [ ] **Step 1:** Install: `npm i -D husky lint-staged && npx husky init`
- [ ] **Step 2:** Set `.husky/pre-commit` contents to:

```sh
npx lint-staged
```

- [ ] **Step 3:** Add to `package.json` (runs the guard whenever a `.vue`/css/token file is staged — fast, no full build in the hook):

```json
  "lint-staged": {
    "{src/**/*.vue,src/assets/*.css}": "vitest run tailwind-token-usage"
  }
```

- [ ] **Step 4:** Test the hook: stage a deliberate violation and try to commit.

```bash
# temporarily add gap-[7px] to any component, then:
git add -A && git commit -m "test: should be blocked"
```
Expected: commit BLOCKED, guard prints the `gap-[7px] → gap-sm` instructive message. Revert the deliberate violation.

- [ ] **Step 5:** Commit: `git add -A && git commit -m "chore: add husky pre-commit running the token guard"`

### Task 4.3: CI workflow

**Files:** Create `.github/workflows/ci.yml`

- [ ] **Step 1:** Create the workflow:

```yaml
name: CI
on:
  push: { branches: [main] }
  pull_request:
jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 20, cache: npm }
      - run: npm ci
      - run: npm run verify
```

- [ ] **Step 2:** Validate locally that `npm ci && npm run verify` passes (the same commands CI runs).
- [ ] **Step 3:** Commit: `git commit -am "ci: run npm run verify on push/PR"`

### ⛔ Phase 4 checkpoint — stop and confirm.

---

## Phase 5: De-rot the token source

### Task 5.1: Remove dead `--app-text-*` tokens and clear default font sizes

**Files:** Modify `src/assets/tokens.css`, `src/assets/main.css`

- [ ] **Step 1:** In `tokens.css`, delete the now-unused `--app-text-2xs/xs-alt/micro/xs/sm/base/lg/xl` size tokens (keep `--glyph-plus`). Confirm zero references first: `grep -rn "app-text-" src` → only the guard test's regex string should match (it references the *pattern*, not a token); no component should.
- [ ] **Step 2:** In `main.css` `@theme`, clear Tailwind's default font sizes so `text-sm` et al. literally don't exist (makes the wrong thing impossible, not just linted):

```css
  --text-*: initial;
```
Place this line at the TOP of the `@theme` block, before the four `--text-title/body/secondary/micro` definitions.

- [ ] **Step 3:** `npm run verify` → PASS. (If any default text utility was still in use, the build/guard surfaces it — fix per mapping.)
- [ ] **Step 4:** Commit: `git commit -am "refactor(tokens): drop dead --app-text-* vars; clear default text sizes"`

### Task 5.2: Comments — intent not inventory; kill the dual source

**Files:** Modify `src/assets/tokens.css`

- [ ] **Step 1:** Delete the header lines claiming the demo is a second source: remove `extracted from UX-REDESIGN-DEMO.html` / `Compare with demo to verify no drift`. Replace with a one-liner: `/* Canonical token source. Utility-generating scales (text, spacing) live in main.css @theme. */`
- [ ] **Step 2:** For every remaining token, ensure the comment states a **role/intent**, not an instance list. Keep rationale comments (the `--app-font-*` / `--app-text-*` namespace-collision note is now partly obsolete — update it to explain why scales moved to `@theme`). No comment should enumerate "which components use this".
- [ ] **Step 3:** `npm run verify` → PASS (comments only; no behavior change).
- [ ] **Step 4:** Commit: `git commit -am "docs(tokens): comments state intent not inventory; single source of truth"`

### ⛔ Phase 5 checkpoint — stop and confirm.

---

## Phase 6: In-context governance rule

### Task 6.1: Add the standing rule to CLAUDE.md

**Files:** Modify `CLAUDE.md`

- [ ] **Step 1:** Under `### 前端交互`, add a sibling subsection:

```markdown
### 设计 token

间距、字号必须用语义 token：间距走 `--spacing-*` 命名档（`gap-sm`/`p-md`，禁止裸 px 与 Tailwind 数字默认档）；字号走 `text-title/body/secondary/micro`（默认 `text-sm` 等已清除）。新增/改阶梯走 PR 说明理由。破例需一行注释 + 显式豁免 + 人工签字。详见 `docs/superpowers/specs/2026-06-21-design-system-consolidation-design.md` §2–3。`src/__tests__/tailwind-token-usage.test.ts` 是可执行护栏，报错含合法替代。
```

- [ ] **Step 2:** Commit: `git commit -am "docs(CLAUDE): require semantic spacing/type tokens (executable guard)"`

### ⛔ Phase 6 checkpoint — final. Run `npm run verify` once more; confirm the design-system consolidation is complete.

---

## Self-review notes (author)

- **Spec coverage:** §2.1 type scale → P1.1 + P3; §2.2 spacing → P1.2 + P3; §2.3 motion → P1.3 + P3.5 step 4; §2.4 radius → P3.3/P3.5; §2.5 raw-Tailwind cluster → P3.5; §3.1 guard+session-gate+instructive-msgs+human-review → P2 + P4; §3.2 @theme single source → P1 + P5; §3.2 escape-hatch+signoff → P3.4 CTA path + P6 rule; §3.3 comments/dual-source → P5.2; §4 naming → P1.1 (deliberate `--text-*`); §6 risks (build-vs-test, sizing exclusion, agent-loose-guard) → Conventions + guard sizing-prefix exclusion + P2 human review.
- **Open decision carried into execution:** SetupScreen CTA 16px → text-body vs sanctioned one-off (P3.4); `text-secondary` color-overload fallback to `--text-meta` (P1.1) — both have explicit default + fallback, not placeholders.
- **Ratchet keeps every commit green** (allowlist + stale-allowlist guard), satisfying TDD red→green per file.
